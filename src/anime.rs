use crate::api::AniList;
use crate::cli::*;
use crate::scraper::*;
use crate::utils::{self, bars, tui};

use anyhow::{bail, Context, Result};
use futures::future::join_all;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::{Client, Url};
use tokio::{fs, io::AsyncWriteExt, task};

use std::path::PathBuf;
use std::process::Command;

#[derive(Default)]
pub struct Manager {
    args: Args,
}

impl Manager {
    pub fn from(args: Args) -> Self {
        Self { args }
    }

    pub async fn run(&self) -> Result<()> {
        if self.args.clean {
            AniList::clean_cache()?
        }

        if self.args.stream {
            self.stream().await?
        } else if self.args.single {
            self.single().await?
        } else {
            self.multi().await?
        }

        Ok(())
    }

    async fn filter_args(&self) -> Result<(Range, ScraperItems)> {
        let args = &self.args;

        let range = match args.range {
            Some(range) => range,
            _ => Range::from((1, 0)),
        };

        // Scrape from archive and find correct url
        let items = match &args.search {
            Some(site) => {
                Scraper::new()
                    .site(site)
                    .query(&args.urls.to_query())
                    .run()
                    .await?
            }
            _ => args
                .urls
                .to_vec()
                .iter()
                .map(|s| ScraperItems::item(s.to_owned(), None))
                .collect::<ScraperItems>(),
        };

        Ok((range, items))
    }

    async fn single(&self) -> Result<()> {
        let args = &self.args;
        let pb = bars::instance_bar();
        let (_, items) = self.filter_args().await?;
        let opts = (args.dir.last().unwrap().to_owned(), args.force, pb);

        Self::download(&items.first().unwrap().url, opts).await
    }

    async fn stream(&self) -> Result<()> {
        let (range, items) = self.filter_args().await?;
        let args = &self.args;

        let urls = if args.single {
            items.iter().map(|a| a.url.clone()).collect::<Vec<_>>()
        } else {
            let item = items.first().unwrap();
            let anime = Anime::builder()
                .url(&item.url)
                .path(args.dir.first().unwrap())
                .id(item.id)
                .range(range)
                .auto(true)
                .build()
                .await?;

            tui::prompt_choices(anime.choices())?
        };

        Command::new("vlc")
            .args(urls)
            .output()
            .context("vlc is needed for streaming")?;

        Ok(())
    }

    async fn multi(&self) -> Result<()> {
        let multi_bars = bars::instance_multi_bars();
        let (range, items) = self.filter_args().await?;
        let args = &self.args;

        let mut pool = vec![];
        for item in &items {
            let mut dir = args.dir.last().unwrap().to_owned();

            let path = if args.auto_dir {
                let subfolder = utils::extract_info(&item.url)?;
                dir.push(subfolder.name);
                dir
            } else {
                let pos = items
                    .iter()
                    .map(|i| i.url.as_str())
                    .position(|u| u == item.url)
                    .unwrap();

                match args.dir.get(pos) {
                    Some(path) => path.to_owned(),
                    _ => dir,
                }
            };

            let anime = Anime::builder()
                .url(item.url.as_str())
                .path(&path)
                .range(range)
                .auto(args.auto_episode || args.interactive)
                .build()
                .await?;

            let path = anime.path();
            let episodes = if args.interactive {
                tui::prompt_choices(anime.choices())?
            } else {
                anime.episodes
            };

            pool.extend(
                episodes
                    .into_iter()
                    .map(|u| {
                        let pb = bars::instance_bar();
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

    async fn download(url: &str, opts: (PathBuf, bool, bars::ProgressBar)) -> Result<()> {
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

        let props = (root, filename.as_str(), overwrite);
        let file = FileDest::from(props).await?;
        if file.size >= source_size {
            bail!("{} already exists", &filename);
        }

        let msg = match utils::extract_info(&filename) {
            Ok(info) => format!("Ep. {:02} {}", info.num, info.name),
            _ => utils::to_title_case(&filename),
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
struct AnimeBuilder {
    auto: bool,
    range: Range,
    path: PathBuf,
    url: String,
    id: Option<u32>,
}

impl AnimeBuilder {
    fn auto(self, auto: bool) -> Self {
        Self { auto, ..self }
    }

    fn id(self, id: Option<u32>) -> Self {
        Self { id, ..self }
    }

    fn range(self, range: Range) -> Self {
        Self { range, ..self }
    }

    fn path(self, path: &PathBuf) -> Self {
        Self {
            path: path.to_owned(),
            ..self
        }
    }

    fn url(self, url: &str) -> Self {
        Self {
            url: url.to_owned(),
            ..self
        }
    }

    async fn build(self) -> Result<Anime> {
        let info = utils::extract_info(&self.url)?;
        let episodes = self.episodes(&info.raw).await?;
        let last_viewed = self.last_viewed().await?;

        Ok(Anime {
            episodes,
            last_viewed,
            path: self.path,
        })
    }

    async fn episodes(&self, url: &str) -> Result<Vec<String>> {
        let ((start, end), auto) = (self.range.extract(), self.auto);

        let num_episodes = if !auto {
            end
        } else {
            let client = Client::new();
            let mut err;
            let mut last;
            let mut counter = 2;

            // Last episode search is an O(log2 n) algorithm:
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

    async fn last_viewed(&self) -> Result<Option<u32>> {
        Ok(match self.id {
            Some(id) => match AniList::new() {
                Some(a) => a.id(id).last_viewed().await?,
                _ => None,
            },
            _ => None,
        })
    }
}

struct Anime {
    last_viewed: Option<u32>,
    episodes: Vec<String>,
    path: PathBuf,
}

impl Anime {
    fn builder() -> AnimeBuilder {
        AnimeBuilder::default()
    }

    fn choices(&self) -> Vec<tui::Choice> {
        self.episodes
            .iter()
            .map(|u| {
                let info = utils::extract_info(u).unwrap();
                let mut name = format!("{} ep. {}", info.name, info.num);

                if let Some(last) = self.last_viewed {
                    if info.num <= last as u32 {
                        name = format!("{} ✔️", name);
                    }
                }

                tui::Choice::from(u.to_string(), name)
            })
            .collect::<Vec<_>>()
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

struct FileDest {
    pub size: u64,
    pub root: PathBuf,
    pub file: PathBuf,
    pub overwrite: bool,
}

type FileProps<'a> = (&'a PathBuf, &'a str, &'a bool);

impl FileDest {
    async fn from<'a>(props: FileProps<'a>) -> Result<Self> {
        let (root, filename, overwrite) = props;

        if !root.exists() {
            fs::create_dir_all(root).await?;
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

    async fn open(&self) -> Result<fs::File> {
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
