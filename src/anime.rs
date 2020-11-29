use crate::cli::*;
use crate::scraper::*;
use crate::utils::{self, bars, tui};

#[cfg(feature = "anilist")]
use crate::api::AniList;

use anyhow::{bail, Context, Result};
use futures::future::join_all;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::{Client, Url};
use tokio::{fs, io::AsyncWriteExt, task};

use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;

enum Action {
    MultiDownload,
    SingleDownload,
    Streaming,
}

impl Action {
    fn parse(args: &Args) -> Self {
        if args.stream {
            Self::Streaming
        } else if args.single {
            Self::SingleDownload
        } else {
            Self::MultiDownload
        }
    }
}

pub struct Manager {
    action: Action,
    args: Args,
    items: ScraperItems,
}

impl Manager {
    pub async fn new(args: Args) -> Result<Self> {
        let action = Action::parse(&args);

        // Scrape from archive and find correct url
        let items = match args.search {
            Some(site) => {
                Scraper::new()
                    .proxy(!args.no_proxy)
                    .query(&args.urls.to_query())
                    .site(site)
                    .run()
                    .await?
            }
            None => args
                .urls
                .to_vec()
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

    pub async fn run(self) -> Result<()> {
        #[cfg(feature = "anilist")]
        if self.args.clean {
            AniList::clean_cache()?
        }

        match self.action {
            Action::Streaming => self.stream().await,
            Action::MultiDownload => self.multi().await,
            Action::SingleDownload => self.single().await,
        }
    }

    async fn single(&self) -> Result<()> {
        let pool = Pool::new();

        for (pos, item) in self.items.iter().enumerate() {
            let url = item.url.clone();
            let opts = (
                utils::get_path(&self.args, &item.url, pos)?,
                self.args.force,
                pool.add_bar(),
            );

            pool.add_task(tokio::spawn(async move {
                print_err!(Self::download(&url, opts).await)
            }))
        }

        pool.join_all().await;

        Ok(())
    }

    async fn stream(&self) -> Result<()> {
        let urls = if self.args.single {
            self.items.iter().map(|a| a.url.clone()).collect::<Vec<_>>()
        } else {
            let item = self.items.first().unwrap();
            let anime = Anime::builder()
                .item(item)
                .path(&self.args.dir.first().unwrap())
                .range(&self.args.range)
                .auto(true)
                .build()
                .await?;

            tui::get_choice(anime.choices())?
        };

        // NOTE: Workaround for streaming in win
        let cmd = match cfg!(windows) {
            true => r"C:\Program Files\VideoLAN\VLC\vlc",
            false => "vlc",
        };

        Command::new(cmd)
            .args(urls)
            .output()
            .context("vlc is needed for streaming")?;

        Ok(())
    }

    async fn multi(&self) -> Result<()> {
        let args = &self.args;

        let pool = Pool::new();
        for (pos, item) in self.items.iter().enumerate() {
            let path = utils::get_path(&args, &item.url, pos)?;

            let anime = Anime::builder()
                .item(item)
                .path(&path)
                .range(&args.range)
                .auto(args.auto_episode || args.interactive)
                .build()
                .await?;

            let episodes = if args.interactive {
                tui::get_choice(anime.choices())?
            } else {
                anime.episodes
            };

            pool.extend(
                episodes
                    .into_iter()
                    .map(|u| {
                        let opts = (path.clone(), args.force, pool.add_bar());

                        tokio::spawn(async move { print_err!(Self::download(&u, opts).await) })
                    })
                    .collect::<TaskPool>(),
            )
        }

        pool.join_all().await;

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
            Err(_) => utils::to_title_case(&filename),
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

type Task = task::JoinHandle<()>;
type TaskPool = Vec<Task>;

struct Pool {
    tasks: RefCell<TaskPool>,
    bars: bars::MultiProgress,
}

impl Pool {
    fn new() -> Self {
        Self {
            tasks: RefCell::new(vec![]),
            bars: bars::instance_multi_bars(),
        }
    }

    fn extend(&self, pool: TaskPool) {
        self.tasks.borrow_mut().extend(pool)
    }

    fn add_task(&self, task: Task) {
        self.tasks.borrow_mut().push(task)
    }

    fn add_bar(&self) -> bars::ProgressBar {
        self.bars.add(bars::instance_bar())
    }

    async fn join_all(self) {
        let bars = self.bars;
        let join_bars = task::spawn_blocking(move || bars.join().unwrap());
        join_all(self.tasks.into_inner()).await;
        join_bars.await.unwrap();
    }
}

#[derive(Default, Debug)]
struct AnimeBuilder {
    auto: bool,
    range: Range<u32>,
    path: PathBuf,
    url: String,
    _id: Option<u32>,
}

impl AnimeBuilder {
    fn auto(self, auto: bool) -> Self {
        Self { auto, ..self }
    }

    fn range(self, range: &Range<u32>) -> Self {
        Self {
            range: range.to_owned(),
            ..self
        }
    }

    fn path(self, path: &PathBuf) -> Self {
        Self {
            path: path.to_owned(),
            ..self
        }
    }

    fn item(self, item: &ScraperItemDetails) -> Self {
        Self {
            url: item.url.to_owned(),
            _id: item.id,
            ..self
        }
    }

    async fn build(self) -> Result<Anime> {
        let info = utils::extract_info(&self.url)?;
        let episodes = self.episodes(&info.raw).await?;
        let _last: Option<u32> = None;

        #[cfg(feature = "anilist")]
        let _last = self.last_viewed().await?;

        Ok(Anime {
            episodes,
            last_viewed: _last,
            _path: self.path,
        })
    }

    async fn episodes(&self, url: &str) -> Result<Vec<String>> {
        let num_episodes = if !self.auto {
            self.range.end
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
                    Ok(_) => counter *= 2,
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

        let episodes = (self.range.start..num_episodes + 1)
            .into_iter()
            .map(|i| gen_url!(url, i))
            .collect::<Vec<_>>();

        if episodes.is_empty() {
            bail!("Unable to download")
        }

        Ok(episodes)
    }

    #[cfg(feature = "anilist")]
    async fn last_viewed(&self) -> Result<Option<u32>> {
        Ok(match self._id {
            Some(id) => match AniList::new() {
                Some(a) => a.id(id).last_viewed().await?,
                None => None,
            },
            None => None,
        })
    }
}

#[derive(Default, Debug)]
struct Anime {
    last_viewed: Option<u32>,
    episodes: Vec<String>,
    _path: PathBuf,
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
                .truncate(true)
                .open(&self.file)
                .await?
        };

        Ok(file)
    }
}
