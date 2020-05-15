use crate::utils::*;

use anyhow::{bail, Context, Result};
use indicatif::ProgressBar;
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, CONTENT_LENGTH, RANGE};
use reqwest::Url;

use std::fs;
use std::io;
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

    pub fn url_episodes(&self) -> Result<Vec<String>> {
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

            match client.head(&url).send()?.error_for_status() {
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

    pub fn download(url: &str, opts: &(PathBuf, bool, ProgressBar)) -> Result<()> {
        let (root, overwrite, pb) = opts;

        let source = WebSource::new(url)?;
        let filename = source.name;

        let file = FileDest::new(root, &filename, overwrite)?;
        if file.size >= source.size {
            bail!("{} already exists", &filename);
        }

        let mut outfile = file.open()?;

        let info = extract_info(&filename)?;
        let msg = format!("Ep. {:02} {}", info.num, info.name);

        let iter_start = file.size;
        let iter_end = source.size;

        pb.set_length(iter_end);
        pb.set_position(iter_start);
        pb.set_message(&msg);

        let client = Client::new();
        for range in PartialRangeIter::new(iter_start, iter_end - 1, CHUNK_SIZE)? {
            let mut response = client
                .get(url)
                .header(RANGE, &range)
                .timeout(std::time::Duration::from_secs(120))
                .send()?
                .error_for_status()?;

            io::copy(&mut response, &mut outfile)?;

            pb.inc(CHUNK_SIZE as u64);
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
    pub fn new(root: &PathBuf, filename: &str, overwrite: &bool) -> Result<Self> {
        if !root.exists() {
            std::fs::create_dir_all(&root)?;
        }

        let mut file = root.clone();
        file.push(filename);

        let size = match file.exists() && !overwrite {
            true => fs::File::open(&file)?.metadata()?.len(),
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

    pub fn open(&self) -> Result<fs::File> {
        let file = if !self.overwrite {
            fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&self.file)?
        } else {
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&self.file)?
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
    pub fn new(str_url: &str) -> Result<Self> {
        let url = Url::parse(str_url)?;
        let name = url
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("tmp.bin")
            .to_owned();

        let client = Client::new();
        let response = client
            .head(str_url)
            .timeout(std::time::Duration::from_secs(120))
            .send()?
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

pub struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: usize,
}

impl PartialRangeIter {
    pub fn new(start: u64, end: u64, buffer_size: usize) -> Result<Self> {
        if buffer_size == 0 {
            bail!("Invalid buffer_size, give a value greater than zero.");
        }

        Ok(Self {
            start,
            end,
            buffer_size,
        })
    }
}

impl Iterator for PartialRangeIter {
    type Item = HeaderValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
            // NOTE(unwrap): `HeaderValue::from_str` will fail only if the value is not made
            // of visible ASCII characters. Since the format string is static and the two
            // values are integers, that can't happen.
            Some(
                HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1)).unwrap(),
            )
        }
    }
}
