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
use crate::anime::{self, Anime, AnimeInfo};
use crate::cli::Args;
use crate::errors::{RemoteError, SystemError};
use crate::file::FileDest;
use crate::scraper::{Scraper, ScraperItems};
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

        let items = if args.watching {
            let anilist = AniList::new(args.anilist_id);

            match anilist.get_watching_list().await {
                Some(list) => {
                    let series = tui::watching_choice(&list)?;
                    let query = series
                        .iter()
                        .map(|s| {
                            s.split_ascii_whitespace()
                                .take(2)
                                .fold(String::new(), |acc, s| acc + " " + s)
                        })
                        .collect::<Vec<_>>();

                    Scraper::new(&query.join(","))
                        .with_proxy(!args.no_proxy)
                        .run()
                        .await?
                }
                _ => bail!(RemoteError::WatchingList),
            }
        } else if utils::is_web_url(&args.entries[0]) {
            args.entries
                .iter()
                .map(|s| AnimeInfo::new(s, None, None))
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

    async fn download(args: Args, items: ScraperItems) -> Result<()> {
        let referrer = &items.referrer;
        let bars = utils::Bars::new();
        let mut pool = vec![];

        for info in items.iter() {
            let last_watched = anime::last_watched(args.anilist_id, info.id).await;
            let mut anime = Anime::new(info, last_watched);

            // if no episode is found, try with the download url
            // if anime.episodes.is_empty() {
            //     let range = args.range.as_ref().cloned().unwrap_or_default();
            //     if let Ok(episodes) = anime::find_episodes(info, referrer, &range).await {
            //         anime.episodes = episodes;
            //         anime.start = *range.start();
            //     }
            // }

            if args.interactive {
                anime.episodes = unroll!(tui::episodes_choice(&anime))
            }

            let path = utils::get_path(&args, &anime.info.url)?;
            for url in anime.episodes {
                let root = path.clone();
                let overwrite = args.force;
                let pb = bars.add_bar();

                let future = async move {
                    let client = Client::new();
                    let filename = utils::parse_filename(&url)?;
                    let source_size = client
                        .head(&url)
                        .header(REFERER, referrer)
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

                    let info = AnimeInfo::new(&url, None, None);
                    let msg = if let Some(inum) = info.num {
                        "Ep. ".to_string() + &zfill!(inum.value, 2) + " " + &info.name
                    } else {
                        info.name
                    };

                    pb.set_position(file.size);
                    pb.set_length(source_size);
                    pb.set_message(msg.clone());
                    let mut source = client
                        .get(url)
                        .header(RANGE, format!("bytes={}-", file.size))
                        .header(REFERER, referrer)
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
            .buffer_unordered(args.dim_buff.max(1))
            .collect::<Vec<_>>()
            .await;

        Ok(())
    }

    async fn streaming(args: Args, items: ScraperItems) -> Result<()> {
        let referrer = &items.referrer;
        let (cmd, cmd_referrer) = match which("mpv") {
            Ok(c) => (c, format!("--referrer={referrer}")),
            _ => (
                which("vlc")
                    .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\VideoLAN\VLC\vlc")),
                format!("--http-referrer={referrer}"),
            ),
        };

        for info in items.iter() {
            let last_watched = anime::last_watched(args.anilist_id, info.id).await;
            let anime = Anime::new(info, last_watched);

            // if no episode is found, try with the download url
            // if anime.episodes.is_empty() {
            //     let range = args.range.as_ref().cloned().unwrap_or_default();
            //     if let Ok(episodes) = anime::find_episodes(info, referrer, &range).await {
            //         anime.episodes = episodes;
            //         anime.start = *range.start();
            //     }
            // }

            let urls = unroll!(tui::episodes_choice(&anime));
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
