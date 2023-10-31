use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use futures::stream::StreamExt;
use owo_colors::OwoColorize;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use reqwest::Client;
use tokio::{fs, io::AsyncWriteExt, process::Command};
use tokio_stream as stream;
use which::which;

#[cfg(feature = "anilist")]
use crate::anilist;

use crate::anilist::WatchingAnime;
use crate::anime::{self, Anime, AnimeInfo};
use crate::cli::Args;
use crate::config::clean_config;
use crate::errors::{RemoteError, SystemError};
use crate::parser;
use crate::scraper::{select_proxy, Scraper, Search, SearchResult};
use crate::tui;

pub struct App;

impl App {
    pub async fn run() -> Result<()> {
        let args = Args::parse();

        #[cfg(feature = "anilist")]
        if args.clean {
            return clean_config();
        }

        let items = if args.watching {
            let list = anilist::get_watching_list(args.anilist_id)
                .await
                .ok_or(RemoteError::WatchingList)?;

            let series = tui::watching_choice(&list)?;
            let search = series.iter().map(|WatchingAnime { title, id, .. }| {
                let string = title
                    .split_ascii_whitespace()
                    .take(2)
                    .fold(String::new(), |acc, s| acc + "+" + s.trim());

                Search {
                    string,
                    id: Some(*id),
                }
            });

            let site = args.site.unwrap_or_default();
            let proxy = select_proxy(args.no_proxy).await;
            Scraper::new(proxy).run(search, site).await?
        } else if parser::is_web_url(&args.entries[0]) {
            args.entries
                .iter()
                .map(|s| {
                    AnimeInfo::new(
                        &to_title_case!(parser::parse_name(s).unwrap()),
                        s,
                        None,
                        None,
                    )
                })
                .collect()
        } else {
            let site = args.site.unwrap_or_default();
            let proxy = select_proxy(args.no_proxy).await;
            let input = &args.entries.join(" ");
            let search = input
                .split(',')
                .map(|s| s.trim().replace(' ', "+"))
                .map(|s| Search {
                    string: s,
                    id: None,
                });

            Scraper::new(proxy).run(search, site).await?
        };

        if args.stream {
            Self::streaming(args, items).await
        } else {
            Self::download(args, items).await
        }
    }

    async fn download(args: Args, items: SearchResult) -> Result<()> {
        let bars = tui::Bars::new();
        let mut pool = vec![];

        for info in items.iter() {
            let last_watched = anime::last_watched(args.anilist_id, info.id).await;
            let mut anime = Anime::new(info, last_watched);

            if args.interactive {
                anime.episodes = unroll!(tui::episodes_choice(&anime))
            }

            let parent = parser::parse_path(&args, &anime.info.url)?;
            for url in anime.episodes {
                let mut path = parent.clone();
                let pb = bars.add_bar();

                let future = async move {
                    let client = Client::new();
                    let filename = parser::parse_filename(&url)?;
                    let source_size = client
                        .head(&url)
                        .header(REFERER, items.referrer)
                        .send()
                        .await?
                        .error_for_status()
                        .context(RemoteError::Download(filename.clone()))?
                        .headers()
                        .get(CONTENT_LENGTH)
                        .and_then(|ct_len| ct_len.to_str().ok())
                        .and_then(|ct_len| ct_len.parse().ok())
                        .unwrap_or_default();

                    let mut dest = {
                        if !path.exists() {
                            fs::create_dir_all(&path).await?;
                        }
                        path.push(&filename);

                        fs::OpenOptions::new()
                            .append(!args.force)
                            .truncate(args.force)
                            .write(args.force)
                            .create(true)
                            .open(path)
                            .await
                            .context(SystemError::FsOpen)?
                    };

                    let file_size = dest.metadata().await?.len();
                    if file_size >= source_size {
                        bail!(SystemError::Overwrite(filename));
                    }

                    let msg = if let Some(inum) = info.num {
                        "Ep. ".to_string() + &zfill!(inum.value, 2) + " " + &info.name
                    } else {
                        info.name.clone()
                    };

                    pb.set_position(file_size);
                    pb.set_length(source_size);
                    pb.set_message(msg.clone());

                    let mut source = client
                        .get(url)
                        .header(RANGE, format!("bytes={}-", file_size))
                        .header(REFERER, items.referrer)
                        .send()
                        .await?
                        .error_for_status()?;
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

    async fn streaming(args: Args, items: SearchResult) -> Result<()> {
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
