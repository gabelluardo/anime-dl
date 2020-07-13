use crate::utils::*;

use anyhow::{bail, Context, Result};
use indicatif::ProgressBar;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::{Client, Url};
use tokio::{fs, io::AsyncWriteExt};

use std::path::PathBuf;

pub struct Anime {
    end: u32,
    start: u32,
    auto: bool,
    url: String,
    path: PathBuf,
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
        let mut episodes = vec![];
        let mut last: u32 = 0;

        let mut error: Vec<u32> = vec![];
        let mut error_counter = 0;
        let mut counter: u32 = self.start;

        let client = Client::new();
        let num_episodes = match self.auto {
            true => u8::max_value() as u32,
            _ => self.end,
        };

        while error_counter < 6 && counter <= num_episodes {
            let num = fix_num_episode(counter);
            let url = self.url.replace(REGEX_VALUE, &num);

            match client.head(&url).send().await?.error_for_status() {
                Err(_) => {
                    error.push(counter);
                    error_counter += 1
                }
                Ok(_) => {
                    episodes.push(url.to_string());
                    last = counter;
                    error_counter = 0;
                }
            }
            counter += 1;
        }

        // TODO: add ability to find different version (es. _v2_, _v000_, ecc)
        error.retain(|&x| x < last);
        if error.len() > 0 {
            format_wrn(&format!(
                "Problems with ep. {:?}, download it manually",
                error
            ));
        }

        Ok(episodes)
    }

    pub async fn download(url: String, opts: (PathBuf, bool, ProgressBar)) -> Result<()> {
        let (root, overwrite, pb) = &opts;
        let url = &url;

        let source = WebSource::new(url).await?;
        let filename = source.name;

        let file = FileDest::new(root, &filename, overwrite).await?;
        if file.size >= source.size {
            bail!("{} already exists", &filename);
        }

        let mut dest = file.open().await?;

        let info = extract_info(&filename)?;
        let msg = format!("Ep. {:02} {}", info.num, info.name);

        pb.set_position(file.size);
        pb.set_length(source.size);
        pb.set_message(&msg);

        let client = Client::new();
        let mut source = client
            .get(url)
            .header(RANGE, format!("bytes={}-", file.size))
            .send()
            .await?;

        while let Some(chunk) = source.chunk().await? {
            dest.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }
        pb.finish_with_message(&format!("{} 👍", msg));

        Ok(())
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

        let size: u64 = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse().ok())
            .unwrap_or_default();

        Ok(Self { name, url, size })
    }
}
