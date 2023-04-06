use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use futures::stream::StreamExt;
use owo_colors::OwoColorize;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use reqwest::Client;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_stream as stream;
use which::which;

#[cfg(feature = "anilist")]
use crate::anilist::AniList;
use crate::anime::{Anime, AnimeInfo, FileDest, InfoNum};
use crate::cli::Args;
use crate::errors::{RemoteError, SystemError};
use crate::scraper::{Scraper, ScraperCollector};
use crate::tui;
use crate::utils;

pub struct App;

impl App {
    pub async fn run() -> Result<()> {
        let args = Args::parse();

        #[cfg(feature = "anilist")]
        if args.clean {
            AniList::clean_cache()?
        }

        let items = if utils::is_web_url(&args.entries[0]) {
            args.entries
                .iter()
                .map(|s| AnimeInfo::new(s, None).unwrap_or_default())
                .collect::<_>()
        } else {
            Scraper::new(&args.entries.join(" "))
                .with_proxy(!args.no_proxy)
                .run()
                .await?
        };

        if args.stream {
            Self::streaming(args, items).await
        } else {
            Self::download(args, items).await
        }
    }

    async fn download(args: Args, items: ScraperCollector) -> Result<()> {
        let referer = &items.referrer;
        let bars = utils::Bars::new();
        let mut pool = vec![];
        for info in items.iter() {
            let path = utils::get_path(&args, &info.url)?;
            let mut anime = Anime::builder()
                .auto(args.auto_episode || args.interactive)
                .client_id(args.anilist_id)
                .info(info)
                .range(args.range.as_ref().unwrap_or_default())
                .referer(referer)
                .path(&path)
                .build()
                .await?;
            if args.interactive {
                anime.episodes = unroll!(tui::get_choice(&anime.choices(), None))
            }
            for url in anime.episodes.into_iter() {
                let root = path.clone();
                let overwrite = args.force;
                let pb = bars.add_bar();

                let future = async move {
                    let client = Client::new();
                    let filename = utils::parse_filename(&url)?;
                    let source_size = client
                        .head(&url)
                        .header(REFERER, referer)
                        .send()
                        .await?
                        .error_for_status()
                        .context(RemoteError::Download(filename.clone()))?
                        .headers()
                        .get(CONTENT_LENGTH)
                        .and_then(|ct_len| ct_len.to_str().ok())
                        .and_then(|ct_len| ct_len.parse().ok())
                        .unwrap_or_default();
                    let props = (root.as_path(), filename.as_str(), overwrite);
                    let file = FileDest::new(props).await?;
                    if file.size >= source_size {
                        bail!(SystemError::Overwrite(filename));
                    }
                    let msg = if let Ok(info) = AnimeInfo::new(&url, None) {
                        let num = info
                            .num
                            .map(|InfoNum { value, .. }| zfill!(value, 2) + " ")
                            .unwrap_or_default();
                        "Ep. ".to_string() + &num + &info.name
                    } else {
                        to_title_case!(filename.split('_').next().unwrap_or_default())
                    };
                    pb.set_position(file.size);
                    pb.set_length(source_size);
                    pb.set_message(msg.clone());
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
                    pb.finish_with_message(msg + " 👍");
                    Ok(())
                };
                pool.push(future);
            }
        }
        stream::iter(pool)
            .buffer_unordered(args.dim_buff)
            .collect::<Vec<_>>()
            .await;

        Ok(())
    }

    async fn streaming(args: Args, items: ScraperCollector) -> Result<()> {
        let referrer = &items.referrer;
        let (cmd, cmd_referrer) = match which("mpv") {
            Ok(c) => (c, format!("--referrer={referrer}")),
            _ => (
                which("vlc")
                    .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\VideoLAN\VLC\vlc")),
                format!("--http-referrer={referrer}"),
            ),
        };
        for item in items.iter() {
            let anime = Anime::builder()
                .auto(true)
                .client_id(args.anilist_id)
                .info(item)
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
                .context(SystemError::MediaPlayer)?;
        }
        Ok(())
    }
}
