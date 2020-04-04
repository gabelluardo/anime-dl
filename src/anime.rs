use failure::bail;
use failure::ResultExt;

use crate::utils::*;
use colored::Colorize;

use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, CONTENT_LENGTH, RANGE};
use reqwest::Url;

use std::path::{Path, PathBuf};

pub type Error<T> = Result<T, failure::Error>;

#[derive(Debug, Clone)]
pub struct Anime {
    url: String,
    start: u32,
    end: u32,
    path: PathBuf,
}

impl Anime {
    pub fn new(url: &str, start: u32, end: u32, path: PathBuf) -> Error<Self> {
        let (url, url_num) = extract(&url)?;

        let end = match end {
            0 => url_num,
            _ => end,
        };

        Ok(Anime {
            url,
            start,
            end,
            path,
        })
    }
    pub fn path(&self) -> String {
        self.path.display().to_string()
    }

    pub fn url_episodes(&self, auto: bool) -> Error<Vec<String>> {
        let mut episodes = vec![];
        let mut last: u32 = 0;

        let mut error: Vec<u32> = vec![];
        let mut error_counter = 0;
        let mut counter: u32 = self.start;

        let client = Client::new();
        let num_episodes = match auto {
            true => u8::max_value() as u32,
            _ => self.end,
        };

        while error_counter < 3 && counter <= num_episodes {
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

        // TODO: making proper error print
        error.retain(|&x| x < last);
        if error.len() > 0 {
            println!(
                "{}",
                format!(
                    "[INFO] Problems with episodes {:?}, download it manually",
                    error
                )
                .yellow()
            );
        }

        Ok(episodes)
    }

    pub fn download(url: &str, path: &str) -> Error<String> {
        let r_url = Url::parse(url)?;
        let filename = r_url
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("tmp.bin");

        let client = Client::new();
        let response = client
            .head(url)
            .send()?
            .error_for_status()
            .context(format!("Unable to download `{}`", filename))?;

        let total_size: u64 = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse().ok())
            .unwrap_or(0);

        let dir = Path::new(&path);
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }

        let mut outfile = std::fs::File::create(format!("{}/{}", path, filename))?;

        println!("[INFO] Downloading {}", filename);

        // println!(
        //     "---\nDownloading {}\nsize = {:?}MB -- {:?}B",
        //     filename,
        //     total_size / 1024u64.pow(2),
        //     total_size,
        // );

        for range in PartialRangeIter::new(0, total_size - 1, CHUNK_SIZE)? {
            let mut response = client
                .get(url)
                .header(RANGE, &range)
                .send()?
                .error_for_status()?;

            // println!("range {:?} total {}", range, total_size);

            std::io::copy(&mut response, &mut outfile)?;
        }

        Ok(filename.to_string())
    }
}

pub struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: usize,
}

impl PartialRangeIter {
    pub fn new(start: u64, end: u64, buffer_size: usize) -> Error<Self> {
        if buffer_size == 0 {
            bail!("Invalid buffer_size, give a value greater than zero.");
        }

        Ok(PartialRangeIter {
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
