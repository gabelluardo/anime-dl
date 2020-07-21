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

        let mut episodes = vec![];
        for i in self.start..num_episodes + 1 {
            episodes.push(gen_url!(self.url, i))
        }

        match episodes.len() {
            0 => bail!("Unable to download"),
            _ => (),
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

    pub async fn download(url: String, opts: (PathBuf, bool, ProgressBar)) -> Result<()> {
        let (root, overwrite, pb) = &opts;
        let url = &url;

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
