use std::path::PathBuf;
use std::process::Stdio;

use futures::stream::StreamExt;
use owo_colors::OwoColorize;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use reqwest::{Client, Url};
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_stream as stream;
use which::which;

#[cfg(feature = "anilist")]
use crate::anilist::AniList;
use crate::anime::{Anime, FileDest};
use crate::cli::Args;
use crate::errors::{Error, Result};
use crate::scraper::{Scraper, ScraperCollector};
use crate::utils::{get_path, tui, Bars, ProgressBar};

#[macro_use]
mod utils;

#[cfg(feature = "anilist")]
mod anilist;

mod anime;
mod cli;
mod errors;
mod scraper;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    #[cfg(feature = "anilist")]
    if args.clean {
        ok!(AniList::clean_cache().await);
    }

    let items = if utils::is_web_url(&args.entries[0]) {
        args.entries
            .iter()
            .map(|s| Scraper::item(s, None))
            .collect::<_>()
    } else {
        let proxy = !args.no_proxy;
        let query = &args.entries.join(" ");

        // currently only one site can be chosen
        // let site = args.site.unwrap_or_default();

        ok!(Scraper::new(query).proxy(proxy).run().await)
    };

    if args.stream {
        ok!(streaming(args, items).await)
    } else {
        ok!(download(args, items).await)
    }
}

async fn download(args: Args, items: ScraperCollector) -> Result<()> {
    let referer = &items.referrer;

    let bars = Bars::new();
    let mut pool = vec![];

    for (pos, item) in items.iter().enumerate() {
        let path = get_path(&args, &item.url, pos)?;

        let mut anime = Anime::builder()
            .auto(args.auto_episode || args.interactive)
            .client_id(args.anilist_id)
            .item(item)
            .range(args.range.as_ref().unwrap_or_default())
            .referer(referer)
            .path(&path)
            .build()
            .await?;

        if args.interactive {
            anime.episodes = unroll!(tui::get_choice(&anime.choices(), None))
        }

        let tasks = anime.episodes.into_iter().map(|u| {
            let opts = (path.clone(), referer.as_str(), args.force, bars.add_bar());

            async move { ok!(download_worker(&u, opts).await) }
        });

        pool.extend(tasks)
    }

    // task::spawn_blocking(move || bars.join().unwrap());
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

    let msg = if let Ok(info) = utils::Info::parse(&filename) {
        let (num, name) = (info.num, info.name);
        num.map(|num| format!("Ep. {num:02} {name}"))
            .unwrap_or(name)
    } else {
        utils::to_title_case(&filename)
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
    let referrer = &items.referrer;

    let (cmd, cmd_referrer) = match which("mpv") {
        Ok(c) => (c, format!("--referrer={referrer}")),
        _ => (
            which("vlc").unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\VideoLAN\VLC\vlc")),
            format!("--http-referrer={referrer}"),
        ),
    };

    for item in items.iter() {
        let anime = Anime::builder()
            .auto(true)
            .client_id(args.anilist_id)
            .item(item)
            .range(args.range.as_ref().unwrap_or_default())
            .referer(referrer)
            .build()
            .await?;

        let urls = unroll!(tui::get_choice(&anime.choices(), None));

        Command::new(&cmd)
            .arg(&cmd_referrer)
            .args(urls)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| Error::MediaPlayer)?;
    }

    Ok(())
}
