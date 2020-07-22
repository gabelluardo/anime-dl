use crate::cli::*;
use crate::utils::*;

use anyhow::{bail, Context, Result};
use futures::future::join_all;
use indicatif::ProgressBar;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
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
        let multi_bars = instance_multi_bars();
        let (start, end, anime_urls) = self.filter_args().await?;

        // TODO: Limit max parallel tasks with `args.max_thread`
        let mut pool = vec![];
        for url in &anime_urls {
            let mut dir = self.args.dir.last().unwrap().to_owned();

            let path = if self.args.auto_dir {
                let subfolder = extract_info(&url)?;

                dir.push(subfolder.name);
                dir
            } else {
                let pos = anime_urls
                    .iter()
                    .map(|u| u.as_str())
                    .position(|u| u == url)
                    .unwrap();

                match self.args.dir.get(pos) {
                    Some(path) => path.to_owned(),
                    _ => dir,
                }
            };

            let opts = (start, end, self.args.auto_episode);
            let anime = Anime::new(url, path, opts)?;
            let urls = anime.url_episodes().await?;

            pool.extend(
                urls.into_iter()
                    .map(|u| {
                        let pb = instance_bar();
                        let opts = (anime.path(), self.args.force, multi_bars.add(pb));

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

    pub async fn url_episodes(&self) -> Result<Vec<String>> {
        let num_episodes = if !self.auto {
            self.end
        } else {
            let client = Client::new();
            // let mut error: Vec<u32> = vec![];
            // let mut error_counter: u32 = 0;
            let mut last_err: u32 = 0;
            let mut counter: u32 = 1;
            let mut last: u32 = 0;

            // TODO: Improve performance of last episode research
            while !(last_err == last + 1) {
                let url = gen_url!(self.url, counter);

                // println!("le={} l={} c={}", last_err, last, counter);

                match client.head(&url).send().await?.error_for_status() {
                    Err(_) => {
                        last_err = counter;
                        // error.push(counter);
                        // error_counter += 1;
                    }
                    Ok(_) => {
                        // episodes.push(url.to_string());
                        last = counter;
                        // error_counter = 0;
                    }
                }
                if last_err == 0 {
                    counter *= 2;
                } else {
                    counter = (last_err + last) / 2
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

        // NOTE: add ability to find different version (es. _v2_, _v000_, ecc)
        // error.retain(|&x| x < last);
        // if error.len() > 0 {
        //     format_wrn(&format!(
        //         "Problems with ep. {:?}, download it manually",
        //         error
        //     ));
        // }

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

pub struct Scraper {
    site: Site,
    query: String,
    client: Client,
}

impl Scraper {
    pub fn new(site: Site, query: String) -> Self {
        Self {
            site,
            query,
            client: Client::new(),
        }
    }

    pub async fn run(&self) -> Result<Vec<String>> {
        // Concat string if is passed with "" in shell
        let query = self.query.replace(" ", "+");

        match self.site {
            Site::AW => self.animeworld(&query).await,
            Site::AS => self.animesaturn(&query).await,
        }
    }

    async fn animeworld(&self, query: &str) -> Result<Vec<String>> {
        let source = "https://www.animeworld.tv/search?keyword=";
        let search_url = format!("{}{}", source, query);

        let fragment = self.parse(&search_url).await?;
        let results = {
            let div = Selector::parse("div.film-list").unwrap();
            let a = Selector::parse("a.name").unwrap();

            fragment
                .select(&div)
                .next()
                .unwrap()
                .select(&a)
                .into_iter()
                .map(|a| {
                    (
                        a.value().attr("href").expect("No link found"),
                        a.first_child()
                            .and_then(|a| a.value().as_text())
                            .expect("No name found") as &str,
                    )
                })
                .collect::<Vec<_>>()
        };

        let choices = prompt_choices(results)?;

        let mut urls = vec![];
        for choice in choices {
            let fragment = self.parse(&choice).await?;
            let results = {
                let a = Selector::parse(r#"a[id="downloadLink"]"#).unwrap();

                fragment
                    .select(&a)
                    .into_iter()
                    .last()
                    .and_then(|a| a.value().attr("href"))
            };

            let url = match results {
                Some(u) => u.to_string(),
                _ => bail!("No link found"),
            };
            urls.push(url);
        }

        Ok(urls)
    }

    async fn animesaturn(&self, query: &str) -> Result<Vec<String>> {
        let source = "https://www.animesaturn.com/animelist?search=";
        let search_url = format!("{}{}", source, query);

        let fragment = self.parse(&search_url).await?;
        let results = {
            let a = Selector::parse("a.badge-archivio").unwrap();

            fragment
                .select(&a)
                .into_iter()
                .map(|a| {
                    (
                        a.value().attr("href").expect("No link found"),
                        a.first_child()
                            .and_then(|a| a.value().as_text())
                            .expect("No name found") as &str,
                    )
                })
                .collect::<Vec<_>>()
        };

        let choices = prompt_choices(results)?;

        let mut urls = vec![];
        for choice in choices {
            let fragment = self.parse(&choice).await?;
            let results = {
                let a = Selector::parse("a.bottone-ep").unwrap();

                fragment
                    .select(&a)
                    .next()
                    .and_then(|a| a.value().attr("href"))
                    .expect("No link found")
            };

            let fragment = self.parse(&results).await?;
            let results = {
                let div = Selector::parse("div.card-body").unwrap();
                let a = Selector::parse("a").unwrap();

                fragment
                    .select(&div)
                    .next()
                    .and_then(|div| div.select(&a).next())
                    .and_then(|a| a.value().attr("href"))
                    .expect("No link found")
            };

            let fragment = self.parse(&results).await?;
            let results = {
                let source = Selector::parse(r#"source[type="video/mp4"]"#).unwrap();

                fragment
                    .select(&source)
                    .next()
                    .and_then(|s| s.value().attr("src"))
            };

            let url = match results {
                Some(u) => match self.client.get(u).send().await?.error_for_status() {
                    Ok(_) => u.to_string(),
                    _ => self.as_change_server(&fragment).await?,
                },
                _ => self.as_change_server(&fragment).await?,
            };
            urls.push(url);
        }

        Ok(urls)
    }

    async fn as_change_server(&self, fragment: &Html) -> Result<String> {
        let results = {
            let div = Selector::parse("div.button").unwrap();
            let a = Selector::parse("a").unwrap();
            let opt = fragment
                .select(&div)
                .next()
                .and_then(|div| div.select(&a).last())
                .and_then(|a| a.value().attr("href"));

            match opt {
                Some(v) => v,
                _ => bail!("No link found"),
            }
        };
        let fragment = self.parse(results).await?;

        let url = {
            let source = Selector::parse(r#"source[type="video/mp4"]"#).unwrap();
            let opt = fragment
                .select(&source)
                .next()
                .and_then(|s| s.value().attr("src"));

            match opt {
                Some(v) => v.to_string(),
                _ => bail!("No link found"),
            }
        };

        Ok(url)
    }

    async fn parse(&self, url: &str) -> Result<Html> {
        let response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()
            .context(format!("Unable to get anime page"))?;

        Ok(Html::parse_fragment(&response.text().await?))
    }
}
