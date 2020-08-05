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
use std::process::Command;

#[derive(Default)]
pub struct Manager {
    args: Args,
}

impl Manager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn args(self, args: Args) -> Self {
        Self { args }
    }

    pub async fn run(&self) -> Result<()> {
        if self.args.stream {
            self.stream().await
        } else if self.args.single {
            self.single().await
        } else {
            self.multi().await
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
                let query = &args.urls.join("+");
                Scraper::new().site(site).query(query).run().await?
            }
            _ => args.urls.to_vec(),
        };

        Ok((start, end, urls))
    }

    async fn single(&self) -> Result<()> {
        let args = &self.args;

        let pb = instance_bar();
        let (_, _, anime_urls) = self.filter_args().await?;

        let opts = (args.dir.last().unwrap().to_owned(), args.force, pb);

        Self::download(&anime_urls.first().unwrap(), opts).await?;
        Ok(())
    }

    async fn stream(&self) -> Result<()> {
        let (start, end, anime_urls) = self.filter_args().await?;
        let args = &self.args;

        let urls = if args.single {
            anime_urls
        } else {
            let url = anime_urls.first().unwrap();
            let path = args.dir.first().unwrap().to_owned();
            let opts = (start, end, true);

            let anime = Anime::new().parse(url, path, opts).await?;

            let episodes = anime
                .iter()
                .map(|u| {
                    let info = extract_info(u).unwrap();

                    (u.to_string(), format!("{} ep. {}", info.name, info.num))
                })
                .collect::<Vec<_>>();

            prompt_choices(episodes)?
        };

        Command::new("vlc")
            .args(urls)
            .output()
            .context("vlc is needed for streaming")?;

        Ok(())
    }

    async fn multi(&self) -> Result<()> {
        let multi_bars = instance_multi_bars();
        let (start, end, anime_urls) = self.filter_args().await?;
        let args = &self.args;

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

            let opts = (start, end, (args.auto_episode || args.interactive));
            let anime = Anime::new().parse(url, path, opts).await?;
            let path = anime.path();

            let episodes = if args.interactive {
                let episodes = anime
                    .iter()
                    .map(|u| {
                        let info = extract_info(u).unwrap();

                        (u.to_string(), format!("{} ep. {}", info.name, info.num))
                    })
                    .collect::<Vec<_>>();

                prompt_choices(episodes)?
            } else {
                anime.into_iter().collect::<Vec<_>>()
            };

            pool.extend(
                episodes
                    .into_iter()
                    .map(|u| {
                        let pb = instance_bar();
                        let opts = (path.clone(), args.force, multi_bars.add(pb));

                        tokio::spawn(async move { print_err!(Self::download(&u, opts).await) })
                    })
                    .collect::<Vec<task::JoinHandle<()>>>(),
            )
        }

        let bars = task::spawn_blocking(move || multi_bars.join().unwrap());

        join_all(pool).await;
        bars.await.unwrap();

        Ok(())
    }

    pub async fn download(url: &str, opts: (PathBuf, bool, ProgressBar)) -> Result<()> {
        let (root, overwrite, pb) = &opts;
        let client = Client::new();

        let filename = Url::parse(url)?
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("tmp.bin")
            .to_owned();

        let source_size = client
            .head(url)
            .send()
            .await?
            .error_for_status()
            .context(format!("Unable to download `{}`", filename))?
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse().ok())
            .unwrap_or_default();

        let file = FileDest::new(root, &filename, overwrite).await?;
        if file.size >= source_size {
            bail!("{} already exists", &filename);
        }

        let msg = match extract_info(&filename) {
            Ok(info) => format!("Ep. {:02} {}", info.num, info.name),
            _ => to_title_case(&filename),
        };

        pb.set_position(file.size);
        pb.set_length(source_size);
        pb.set_message(&msg);

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

#[derive(Default)]
struct Anime {
    path: PathBuf,
    episodes: Vec<String>,
}

struct AnimeIntoIterator {
    iter: ::std::vec::IntoIter<String>,
}

impl<'a> IntoIterator for Anime {
    type Item = String;
    type IntoIter = AnimeIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        AnimeIntoIterator {
            iter: self.episodes.clone().into_iter(),
        }
    }
}

impl<'a> Iterator for AnimeIntoIterator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

struct AnimeIterator<'a> {
    iter: ::std::slice::Iter<'a, String>,
}

impl<'a> IntoIterator for &'a Anime {
    type Item = &'a String;
    type IntoIter = AnimeIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AnimeIterator {
            iter: self.episodes.iter(),
        }
    }
}

impl<'a> Iterator for AnimeIterator<'a> {
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl Anime {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn parse(self, url: &str, path: PathBuf, opts: (u32, u32, bool)) -> Result<Self> {
        let (start, end, auto) = opts;
        let info = extract_info(&url)?;

        let end = match end {
            0 => info.num,
            _ => end,
        };

        let episodes = Self::episodes(&info.raw, (start, end, auto)).await?;

        Ok(Self { path, episodes })
    }

    async fn episodes(url: &str, opts: (u32, u32, bool)) -> Result<Vec<String>> {
        let (start, end, auto) = opts;

        let num_episodes = if !auto {
            end
        } else {
            let client = Client::new();
            let mut err;
            let mut last;
            let mut counter = 2;

            // Last episode search is an O(2log n) algorithm:
            // first loop finds a possible least upper bound [O(log2 n)]
            // second loop finds the real upper bound with a binary search [O(log2 n)]
            loop {
                let url = gen_url!(url, counter);

                err = counter;
                last = counter / 2;
                match client.head(&url).send().await?.error_for_status() {
                    Err(_) => break,
                    _ => counter *= 2,
                }
            }

            while !(err == last + 1) {
                counter = (err + last) / 2;
                let url = gen_url!(url, counter);

                match client.head(&url).send().await?.error_for_status() {
                    Ok(_) => last = counter,
                    Err(_) => err = counter,
                }
            }
            last
        };

        let episodes = (start..num_episodes + 1)
            .into_iter()
            .map(|i| gen_url!(url, i))
            .collect::<Vec<_>>();

        if episodes.is_empty() {
            bail!("Unable to download")
        }

        Ok(episodes)
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn iter(&self) -> AnimeIterator {
        self.into_iter()
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
            fs::create_dir_all(&root).await?;
        }

        let mut file = root.clone();
        file.push(filename);

        let size = match file.exists() && !overwrite {
            true => fs::File::open(&file).await?.metadata().await?.len(),
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
