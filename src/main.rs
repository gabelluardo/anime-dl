#[macro_use]
mod utils;

mod anime;
mod api;
mod cli;
mod scraper;

use anime::*;
use cli::*;
use utils::*;

use futures::stream::StreamExt;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use reqwest::{Client, Url};
use tokio::{io::AsyncWriteExt, task};
use tokio_stream::{self as stream};

use std::path::PathBuf;
use std::process::Command;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => (),
        Err(e) => bunt::eprintln!("{$red}[ERR] {}{/$}", e),
    }
}

async fn run() -> Result<()> {
    Manager::new(Args::parse()).await?.run().await
}

enum Action {
    Download,
    Streaming,
    SingleDownload,
}

impl Action {
    fn parse(args: &Args) -> Self {
        if args.stream {
            Self::Streaming
        } else if args.single {
            Self::SingleDownload
        } else {
            Self::Download
        }
    }
}

struct Manager {
    action: Action,
    args: Args,
    items: ScraperItems,
}

impl Manager {
    async fn new(args: Args) -> Result<Self> {
        let action = Action::parse(&args);

        #[cfg(feature = "anilist")]
        if args.clean {
            AniList::clean_cache()?
        }

        // Scrape from archive and find correct url
        let items = match args.search {
            Some(site) => {
                Scraper::new()
                    .proxy(!args.no_proxy)
                    .query(&args.entries.join("+"))
                    .site(site)
                    .run()
                    .await?
            }
            None => args
                .entries
                .iter()
                .map(|s| ScraperItems::item(s.to_owned(), None))
                .collect::<_>(),
        };

        Ok(Self {
            action,
            args,
            items,
        })
    }

    async fn run(self) -> Result<()> {
        match self.action {
            Action::Download => self.multi().await,
            Action::Streaming => self.stream().await,
            Action::SingleDownload => self.single().await,
        }
    }

    // NOTE: Deprecated since 1.2.0
    async fn single(&self) -> Result<()> {
        bail!("`-O` is deprecated since 1.2.0 release")
    }

    async fn stream(&self) -> Result<()> {
        let referer = format!("--http-referrer={}", self.items.referer);
        let item = self.items.first().unwrap();
        let anime = Anime::builder()
            .item(item)
            .referer(&self.items.referer)
            .range(self.args.range.as_ref().unwrap_or_default())
            .auto(true)
            .build()
            .await?;

        let urls = tui::get_choice(anime.choices())?;

        // NOTE: Workaround for streaming in Windows
        let cmd = match cfg!(windows) {
            true => r"C:\Program Files\VideoLAN\VLC\vlc",
            false => "vlc",
        };

        Command::new(cmd)
            .arg(referer)
            .args(urls)
            .output()
            .context("vlc is needed for streaming")?;

        Ok(())
    }

    async fn multi(&self) -> Result<()> {
        let args = &self.args;
        let referer = &self.items.referer;

        let bars = Bars::new();
        let mut pool = vec![];

        for (pos, item) in self.items.iter().enumerate() {
            let path = utils::get_path(args, &item.url, pos)?;

            let mut anime = Anime::builder()
                .item(item)
                .path(&path)
                .referer(&self.items.referer)
                .range(args.range.as_ref().unwrap_or_default())
                .auto(args.auto_episode || args.interactive)
                .build()
                .await?;

            if args.interactive {
                anime.episodes = tui::get_choice(anime.choices())?
            }

            pool.extend(anime.episodes.into_iter().map(|u| {
                let opts = (path.clone(), referer.as_str(), args.force, bars.add_bar());

                async move { print_err!(Self::worker(&u, opts).await) }
            }))
        }

        task::spawn_blocking(move || bars.join().unwrap());
        stream::iter(pool)
            .buffer_unordered(args.dim_buff)
            .collect::<Vec<_>>()
            .await;

        Ok(())
    }

    async fn worker(url: &str, opts: (PathBuf, &str, bool, bars::ProgressBar)) -> Result<()> {
        let (root, referer, overwrite, pb) = opts;
        let client = Client::new();

        let filename = Url::parse(url)?
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
            .context(format!("Unable to download `{}`", filename))?
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse().ok())
            .unwrap_or_default();

        let props = (&root, filename.as_str(), &overwrite);
        let file = FileDest::new(props).await?;
        if file.size >= source_size {
            bail!("{} already exists", &filename);
        }

        let msg = match utils::extract_info(&filename) {
            Err(_) => utils::to_title_case(&filename),
            Ok(info) => info
                .num
                .map(|num| format!("Ep. {:02} {}", num, info.name))
                .unwrap_or(info.name),
        };

        pb.set_position(file.size);
        pb.set_length(source_size);
        pb.set_message(&msg);

        let mut source = client
            .get(url)
            .header(RANGE, format!("bytes={}-", file.size))
            .header(REFERER, referer)
            .send()
            .await?
            .error_for_status()
            .context("Unable get data from source")?;

        let mut dest = file.open().await?;
        while let Some(chunk) = source.chunk().await? {
            dest.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }
        pb.finish_with_message(&format!("{} 👍", msg));

        Ok(())
    }
}
