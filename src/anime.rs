#[cfg(feature = "anilist")]
pub use crate::api::AniList;
pub use crate::scraper::*;

use crate::utils::{self, *};

use reqwest::header::REFERER;
use reqwest::Client;

use tokio::fs;

use std::path::PathBuf;

#[derive(Default, Debug)]
pub struct AnimeBuilder {
    auto: bool,
    client_id: Option<u32>,
    id: Option<u32>,
    path: PathBuf,
    range: Range<u32>,
    referer: String,
    url: String,
}

impl AnimeBuilder {
    pub fn auto(mut self, auto: bool) -> Self {
        self.auto = auto;
        self
    }

    pub fn client_id(mut self, client_id: Option<u32>) -> Self {
        self.client_id = client_id;
        self
    }

    pub fn item(mut self, item: &ScraperItem) -> Self {
        self.id = item.id;
        self.url = item.url.to_owned();
        self
    }

    pub fn range(mut self, range: &Range<u32>) -> Self {
        self.range = range.to_owned();
        self
    }

    pub fn path(mut self, path: &PathBuf) -> Self {
        self.path = path.to_owned();
        self
    }

    pub fn referer(mut self, referer: &str) -> Self {
        self.referer = referer.to_string();
        self
    }

    pub async fn build(mut self) -> Result<Anime> {
        let info = utils::extract_info(&self.url)?;
        let episodes = match info.num {
            Some(_) => self.episodes(&info.raw).await?,
            _ => vec![info.raw],
        };

        Ok(Anime {
            episodes,
            last_viewed: self.last_viewed().await?,
            path: self.path,
        })
    }

    async fn episodes(&mut self, url: &str) -> Result<Vec<String>> {
        if self.auto {
            // Last episode search is an O(log2 n) algorithm:
            // first loop finds a possible least upper bound [O(log2 n)]
            // second loop finds the real upper bound with a binary search [O(log2 n)]

            let client = Client::new();
            let mut err;
            let mut last;
            let mut counter = 2;

            loop {
                err = counter;
                last = counter / 2;

                match client
                    .head(&gen_url!(url, counter))
                    .header(REFERER, &self.referer)
                    .send()
                    .await?
                    .error_for_status()
                {
                    Ok(_) => counter *= 2,
                    Err(_) => break,
                }
            }

            while err != last + 1 {
                counter = (err + last) / 2;

                match client
                    .head(&gen_url!(url, counter))
                    .header(REFERER, &self.referer)
                    .send()
                    .await?
                    .error_for_status()
                {
                    Ok(_) => last = counter,
                    Err(_) => err = counter,
                }
            }

            let first = match self.range.start() {
                // Check if episode 0 is available
                1 => match client
                    .head(&gen_url!(url, 0))
                    .header(REFERER, &self.referer)
                    .send()
                    .await?
                    .error_for_status()
                {
                    Ok(_) => 0,
                    Err(_) => 1,
                },
                _ => *self.range.start(),
            };

            self.range = Range::new(first, last)
        }

        if self.range.is_empty() {
            bail!("Unable to download")
        }

        let episodes = self
            .range
            .expand()
            .map(|i| gen_url!(url, i))
            .collect::<Vec<_>>();

        Ok(episodes)
    }

    #[cfg(feature = "anilist")]
    async fn last_viewed(&self) -> Result<Option<u32>> {
        let anilist = AniList::builder()
            .anime_id(self.id)
            .client_id(self.client_id)
            .build()
            .await;

        match anilist {
            Ok(a) => a.last_viewed().await,
            _ => Ok(None),
        }
    }

    #[cfg(not(feature = "anilist"))]
    async fn last_viewed(&self) -> Result<Option<u32>> {
        Ok(None)
    }
}

#[derive(Default, Debug)]
pub struct Anime {
    pub last_viewed: Option<u32>,
    pub episodes: Vec<String>,
    pub path: PathBuf,
}

impl Anime {
    pub fn builder() -> AnimeBuilder {
        AnimeBuilder::default()
    }

    pub fn choices(&self) -> Vec<tui::Choice> {
        self.episodes
            .iter()
            .map(|u| {
                let info = utils::extract_info(u).unwrap();
                let msg = match info.num {
                    Some(num) => {
                        let mut name = format!("{} ep. {}", info.name, num);

                        if info.num <= self.last_viewed {
                            name = format!("{} ✔️", name)
                        }
                        name
                    }
                    _ => utils::extract_name(u).unwrap(),
                };

                tui::Choice::new(u.to_string(), msg)
            })
            .collect::<Vec<_>>()
    }
}

pub struct FileDest {
    pub size: u64,
    pub root: PathBuf,
    pub file: PathBuf,
    pub overwrite: bool,
}

type FileProps<'a> = (&'a PathBuf, &'a str, bool);

impl FileDest {
    pub async fn new(props: FileProps<'_>) -> Result<Self> {
        let (root, filename, overwrite) = props;

        if !root.exists() {
            fs::create_dir_all(root).await?;
        }

        let mut file = root.clone();
        file.push(filename);

        let size = match file.exists() && !overwrite {
            true => fs::File::open(&file).await?.metadata().await?.len(),
            false => 0,
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
                .truncate(true)
                .open(&self.file)
                .await?
        };

        Ok(file)
    }
}
