use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use futures::stream::StreamExt;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use reqwest::Client;
use tokio::{fs, io::AsyncWriteExt, process::Command};
use tokio_stream as stream;
use which::which;

#[cfg(feature = "anilist")]
use crate::anilist;

use crate::anilist::WatchingAnime;
use crate::anime::{Anime, AnimeInfo, InfoNum};
use crate::cli::Args;
use crate::config::clean_config;
use crate::errors::{RemoteError, SystemError};
use crate::parser;
use crate::scraper::{select_proxy, Scraper, Search};
use crate::tui;

pub struct App;

impl App {
    pub async fn run() -> Result<()> {
        let args = Args::parse();

        #[cfg(feature = "anilist")]
        if args.clean {
            return clean_config();
        }

        let (vec_anime, referrer) = if args.watching {
            let mut series = anilist::get_watching_list(args.anilist_id)
                .await
                .ok_or(RemoteError::WatchingList)?;
            tui::watching_choice(&mut series)?;

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
            let (vec_info, referrer) = Scraper::new(proxy).run(search, site).await?;
            let items = vec_info.into_iter().map(|info| Anime::new(&info)).collect();

            (items, referrer)
        } else if parser::is_web_url(&args.entries[0]) {
            let items = args
                .entries
                .iter()
                .map(|s| {
                    let name = to_title_case!(parser::parse_name(s).unwrap());
                    let info = AnimeInfo::new(&name, s, None, None);

                    Anime::new(&info)
                })
                .collect();

            (items, None)
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

            let (vec_info, referrer) = Scraper::new(proxy).run(search, site).await?;
            let items = vec_info.into_iter().map(|info| Anime::new(&info)).collect();

            (items, referrer)
        };

        if args.stream {
            Self::streaming(args, vec_anime, referrer).await
        } else {
            Self::download(args, vec_anime, referrer).await
        }
    }

    async fn download(args: Args, items: Vec<Anime>, referrer: Option<&'static str>) -> Result<()> {
        let bars = tui::Bars::new();
        let mut pool = vec![];

        for mut anime in items.into_iter() {
            if args.interactive {
                tui::episodes_choice(&mut anime)?;
            } else {
                anime.expand();
            }

            let parent = parser::parse_path(&args, &anime.info.url)?;
            for (i, url) in anime.episodes.into_iter().enumerate() {
                let mut path = parent.clone();
                let pb = bars.add_bar();
                let info = anime.info.clone();

                let future = async move {
                    let client = Client::new();
                    let filename = parser::parse_filename(&url)?;
                    let source_size = client
                        .head(&url)
                        .header(REFERER, referrer.unwrap_or_default())
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

                    let msg = if let Some(InfoNum { value, alignment }) = info.num {
                        "Ep. ".to_string() + &zfill!(value + i as u32, alignment) + " " + &info.name
                    } else {
                        info.name.clone()
                    };

                    pb.set_position(file_size);
                    pb.set_length(source_size);
                    pb.set_message(msg);

                    let mut source = client
                        .get(url)
                        .header(RANGE, format!("bytes={}-", file_size))
                        .header(REFERER, referrer.unwrap_or_default())
                        .send()
                        .await?
                        .error_for_status()?;
                    while let Some(chunk) = source.chunk().await? {
                        dest.write_all(&chunk).await?;
                        pb.inc(chunk.len() as u64);
                    }
                    pb.finish_with_message(pb.message() + " 👍");

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

    async fn streaming(
        _args: Args,
        items: Vec<Anime>,
        referrer: Option<&'static str>,
    ) -> Result<()> {
        let referrer = referrer.unwrap_or_default();
        let (cmd, cmd_referrer) = match which("mpv") {
            Ok(c) => (c, format!("--referrer={referrer}")),
            _ => (
                which("vlc")
                    .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\VideoLAN\VLC\vlc")),
                format!("--http-referrer={referrer}"),
            ),
        };

        let mut episodes = vec![];
        for mut anime in items.into_iter() {
            tui::episodes_choice(&mut anime)?;

            episodes.extend(anime.episodes);
        }

        Command::new(&cmd)
            .arg(&cmd_referrer)
            .args(episodes)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context(SystemError::MediaPlayer)?;

        Ok(())
    }
}
