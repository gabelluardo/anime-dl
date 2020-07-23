use crate::cli::*;
use crate::scraper::Scraper;
use crate::utils::*;

use anyhow::{bail, Context, Result};
use futures::future::join_all;
use indicatif::ProgressBar;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::{Client, Url};
use tokio::task;
use tokio::{fs, io::AsyncWriteExt};

use std::path::PathBuf;

pub struct Manager {
    args: Args,
}

impl Manager {
    pub fn new(args: Args) -> Self {
        Self { args }
    }

    pub async fn run(&self) -> Result<()> {
        match self.args.single {
            true => self.single().await,
            _ => self.multi().await,
        }
    }

    async fn filter_args(&self) -> Result<(u32, u32, Vec<String>)> {
        let args = &self.args;

        let (start, end) = match &args.range {
            Some(range) => range.extract(),
            _ => (1, 0),
        };

        // Scrape from archive and find correct url
        let urls = match &args.search {
            Some(site) => {
                let query = args.urls.join("+");
                Scraper::new(site.to_owned(), query).run().await?
            }
            _ => args.urls.to_vec(),
        };

        Ok((start, end, urls))
    }

    async fn single(&self) -> Result<()> {
        let pb = instance_bar();
        let (_, _, anime_urls) = self.filter_args().await?;

        let opts = (
            self.args.dir.last().unwrap().to_owned(),
            self.args.force,
            pb,
        );

        Self::download(&anime_urls[0], opts).await?;
        Ok(())
    }

    async fn multi(&self) -> Result<()> {
        let args = &self.args;
        let multi_bars = instance_multi_bars();
        let (start, end, anime_urls) = self.filter_args().await?;

        let mut pool = vec![];
        for url in &anime_urls {
            let mut dir = args.dir.last().unwrap().to_owned();

            let path = if args.auto_dir {
                let subfolder = extract_info(&url)?;

                dir.push(subfolder.name);
                dir
            } else {
                let pos = anime_urls
                    .iter()
                    .map(|u| u.as_str())
                    .position(|u| u == url)
                    .unwrap();

                match args.dir.get(pos) {
                    Some(path) => path.to_owned(),
                    _ => dir,
                }
            };

            let opts = (start, end, args.auto_episode);
            let anime = Anime::new(url, path, opts)?;
            let urls = anime.episodes().await?;

            pool.extend(
                urls.into_iter()
                    .map(|u| {
                        let pb = instance_bar();
                        let opts = (anime.path(), args.force, multi_bars.add(pb));

                        tokio::spawn(async move { Self::download(&u, opts).await })
                    })
                    .collect::<Vec<task::JoinHandle<Result<()>>>>(),
            )
        }

        let bars = task::spawn_blocking(move || multi_bars.join().unwrap());

        join_all(pool).await;
        bars.await.unwrap();

        Ok(())
    }

    pub async fn download(url: &str, opts: (PathBuf, bool, ProgressBar)) -> Result<()> {
        let (root, overwrite, pb) = &opts;

        let source = WebSource::new(url).await?;
        let filename = source.name;

        let file = FileDest::new(root, &filename, overwrite).await?;
        if file.size >= source.size {
            bail!("{} already exists", &filename);
        }

        let msg = match extract_info(&filename) {
            Ok(info) => format!("Ep. {:02} {}", info.num, info.name),
            _ => to_title_case(&filename),
        };

        pb.set_position(file.size);
        pb.set_length(source.size);
        pb.set_message(&msg);

        let client = Client::new();
        let mut source = client
            .get(url)
            .header(RANGE, format!("bytes={}-", file.size))
            .send()
            .await?
            .error_for_status()
            .context(format!("Unable get data from source"))?;

        let mut dest = file.open().await?;
        while let Some(chunk) = source.chunk().await? {
            dest.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }
        pb.finish_with_message(&format!("{} 👍", msg));

        Ok(())
    }
}

pub struct Anime {
    auto: bool,
    end: u32,
    start: u32,
    path: PathBuf,
    url: String,
}

impl Anime {
    pub fn new(url: &str, path: PathBuf, opts: (u32, u32, bool)) -> Result<Self> {
        let (start, end, auto) = opts;
        let info = extract_info(&url)?;

        let end = match end {
            0 => info.num,
            _ => end,
        };

        Ok(Self {
            end,
            path,
            auto,
            start,
            url: info.raw,
        })
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub async fn episodes(&self) -> Result<Vec<String>> {
        let num_episodes = if !self.auto {
            self.end
        } else {
            let client = Client::new();
            let mut err;
            let mut last;
            let mut counter = 2;

            // Last episode search is an O(2log n) algorithm:
            // first loop finds a possible least upper bound [O(log2 n)]
            // second loop finds the real upper bound with a binary search [O(log2 n)]
            loop {
                let url = gen_url!(self.url, counter);

                err = counter;
                last = counter / 2;
                match client.head(&url).send().await?.error_for_status() {
                    Err(_) => break,
                    _ => counter *= 2,
                }
            }

            while !(err == last + 1) {
                counter = (err + last) / 2;
                let url = gen_url!(self.url, counter);

                match client.head(&url).send().await?.error_for_status() {
                    Ok(_) => last = counter,
                    Err(_) => err = counter,
                }
            }
            last
        };

        let episodes = (self.start..num_episodes + 1)
            .into_iter()
            .map(|i| gen_url!(self.url, i))
            .collect::<Vec<_>>();

        if episodes.is_empty() {
            bail!("Unable to download")
        }

        Ok(episodes)
    }
}

pub struct FileDest {
    pub size: u64,
    pub root: PathBuf,
    pub file: PathBuf,
    pub overwrite: bool,
}

impl FileDest {
    pub async fn new(root: &PathBuf, filename: &str, overwrite: &bool) -> Result<Self> {
        if !root.exists() {
            std::fs::create_dir_all(&root)?;
        }

        let mut file = root.clone();
        file.push(filename);

        let size = match file.exists() && !overwrite {
            true => std::fs::File::open(&file)?.metadata()?.len(),
            _ => 0,
        };

        let root = root.to_owned();
        let overwrite = overwrite.to_owned();

        Ok(Self {
            root,
            file,
            size,
            overwrite,
        })
    }

    pub async fn open(&self) -> Result<fs::File> {
        let file = if !self.overwrite {
            fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&self.file)
                .await?
        } else {
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&self.file)
                .await?
        };

        Ok(file)
    }
}

pub struct WebSource {
    pub url: Url,
    pub size: u64,
    pub name: String,
}

impl WebSource {
    pub async fn new(str_url: &str) -> Result<Self> {
        let url = Url::parse(str_url)?;
        let name = url
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("tmp.bin")
            .to_owned();

        let client = Client::new();
        let response = client
            .head(str_url)
            .send()
            .await?
            .error_for_status()
            .context(format!("Unable to download `{}`", name))?;

        let size = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse().ok())
            .unwrap_or_default();

        Ok(Self { name, url, size })
    }
}
