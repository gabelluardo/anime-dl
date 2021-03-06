use std::path::PathBuf;
use std::process::Stdio;

use futures::stream::StreamExt;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use reqwest::{Client, Url};
use tokio::{io::AsyncWriteExt, process::Command, task};
use tokio_stream as stream;

use crate::anime::{AniList, Anime, FileDest};
use crate::cli::Args;
use crate::errors::{Error, Result};
use crate::scraper::{Scraper, ScraperCollector};
use crate::utils::{get_path, tui, Bars, ProgressBar};

#[macro_use]
mod utils;

mod anime;
mod api;
mod cli;
mod errors;
mod scraper;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    #[cfg(feature = "anilist")]
    if args.clean {
        ok!(AniList::clean_cache().await)
    }

    let items = match args.search {
        Some(site) => {
            let proxy = !args.no_proxy;
            let query = &args.entries.join(" ");

            ok!(Scraper::new(proxy, query, site).run().await)
        }
        None => args
            .entries
            .iter()
            .map(|s| Scraper::item(s, None))
            .collect::<_>(),
    };

    let res = if args.stream {
        streaming(args, items).await
    } else {
        download(args, items).await
    };

    ok!(res)
}

async fn download(args: Args, items: ScraperCollector) -> Result<()> {
    let referer = &items.referer;

    let bars = Bars::new();
    let mut pool = vec![];

    for (pos, item) in items.iter().enumerate() {
        let path = get_path(&args, &item.url, pos)?;

        let mut anime = Anime::builder()
            .auto(args.auto_episode || args.interactive)
            .client_id(args.animedl_id)
            .item(item)
            .range(args.range.as_ref().unwrap_or_default())
            .referer(referer)
            .path(&path)
            .build()
            .await?;

        if args.interactive {
            anime.episodes = unroll!(tui::get_choice(anime.choices(), None).await)
        }

        let tasks = anime.episodes.into_iter().map(|u| {
            let opts = (path.clone(), referer.as_str(), args.force, bars.add_bar());

            async move { ok!(download_worker(&u, opts).await) }
        });

        pool.extend(tasks)
    }

    task::spawn_blocking(move || bars.join().unwrap());
    stream::iter(pool)
        .buffer_unordered(args.dim_buff)
        .collect::<Vec<_>>()
        .await;

    Ok(())
}

async fn download_worker(url: &str, opts: (PathBuf, &str, bool, ProgressBar)) -> Result<()> {
    let (root, referer, overwrite, pb) = opts;
    let client = Client::new();

    let filename = Url::parse(url)
        .map_err(|_| Error::InvalidUrl)?
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("tmp.bin")
        .to_owned();

    let source_size = client
        .head(url)
        .header(REFERER, referer)
        .send()
        .await?
        .error_for_status()
        .map_err(|_| Error::Download(filename.clone()))?
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok())
        .unwrap_or_default();

    let props = (root.as_path(), filename.as_str(), overwrite);
    let file = FileDest::new(props).await?;
    if file.size >= source_size {
        bail!(Error::Overwrite(filename));
    }

    let msg = match utils::extract_info(&filename) {
        Err(_) => utils::to_title_case(&filename),
        Ok(info) => info
            .num
            .map(|num| format!("Ep. {:02} {}", num, info.name))
            .unwrap_or(info.name),
    };
    let completed = format!("{} 👍", &msg);

    pb.set_position(file.size);
    pb.set_length(source_size);
    pb.set_message(msg);

    let mut source = client
        .get(url)
        .header(RANGE, format!("bytes={}-", file.size))
        .header(REFERER, referer)
        .send()
        .await?
        .error_for_status()?;

    let mut dest = file.open().await?;
    while let Some(chunk) = source.chunk().await? {
        dest.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message(completed);

    Ok(())
}

async fn streaming(args: Args, items: ScraperCollector) -> Result<()> {
    let referer = format!("--http-referrer={}", items.referer);

    for item in items.iter() {
        let anime = Anime::builder()
            .auto(true)
            .client_id(args.animedl_id)
            .item(item)
            .range(args.range.as_ref().unwrap_or_default())
            .referer(&items.referer)
            .build()
            .await?;

        let urls = unroll!(tui::get_choice(anime.choices(), None).await);

        // NOTE: Workaround for streaming in Windows
        let cmd = match cfg!(windows) {
            true => r"C:\Program Files\VideoLAN\VLC\vlc",
            false => "vlc",
        };

        Command::new(cmd)
            .arg(&referer)
            .args(urls)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| Error::Vlc)?;
    }

    Ok(())
}
