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
use tokio::{io::AsyncWriteExt, process::Command, task};
use tokio_stream::{self as stream};

use std::path::PathBuf;
use std::process::Stdio;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let m = Manager::new(args);

    print_err!(m.run().await)
}

#[derive(Default)]
struct Manager {
    args: Args,
    items: ScraperItems,
}

impl Manager {
    fn new(args: Args) -> Self {
        Self {
            args,
            ..Self::default()
        }
    }

    async fn run(mut self) -> Result<()> {
        #[cfg(feature = "anilist")]
        if self.args.clean {
            AniList::clean_cache().await?
        }

        // Scrape from archive and find correct url
        self.items = match self.args.search {
            Some(site) => {
                Scraper::new()
                    .proxy(!self.args.no_proxy)
                    .query(&self.args.entries.join("+"))
                    .site(site)
                    .run()
                    .await?
            }
            None => self
                .args
                .entries
                .iter()
                .map(|s| ScraperItems::item(s.to_owned(), None))
                .collect::<_>(),
        };

        if self.args.stream {
            self.stream().await
        } else {
            self.download().await
        }
    }

    async fn stream(&self) -> Result<()> {
        let referer = format!("--http-referrer={}", self.items.referer);
        let item = self.items.first().unwrap();
        let anime = Anime::builder()
            .auto(true)
            .client_id(self.args.animedl_id)
            .item(item)
            .range(self.args.range.as_ref().unwrap_or_default())
            .referer(&self.items.referer)
            .build()
            .await?;

        let urls = tui::get_choice(anime.choices()).await?;

        // NOTE: Workaround for streaming in Windows
        let cmd = match cfg!(windows) {
            true => r"C:\Program Files\VideoLAN\VLC\vlc",
            false => "vlc",
        };

        Command::new(cmd)
            .arg(referer)
            .args(urls)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("vlc is needed for streaming")?;

        Ok(())
    }

    async fn download(&self) -> Result<()> {
        let args = &self.args;
        let referer = &self.items.referer;

        let bars = Bars::new();
        let mut pool = vec![];

        for (pos, item) in self.items.iter().enumerate() {
            let path = utils::get_path(args, &item.url, pos)?;

            let mut anime = Anime::builder()
                .auto(args.auto_episode || args.interactive)
                .client_id(self.args.animedl_id)
                .item(item)
                .range(args.range.as_ref().unwrap_or_default())
                .referer(&self.items.referer)
                .path(&path)
                .build()
                .await?;

            if args.interactive {
                anime.episodes = tui::get_choice(anime.choices()).await?
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

        let props = (&root, filename.as_str(), overwrite);
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
