use failure::bail;
use futures::stream::TryStreamExt;

use reqwest::header::{HeaderValue, CONTENT_LENGTH, RANGE};
use reqwest::Url;

use regex::Regex;

use std::path::{Path, PathBuf};

use tokio_util::compat::FuturesAsyncReadCompatExt;

pub type Error<T> = Result<T, failure::Error>;

// 1024^2 = 1MB
const CHUNK_SIZE: usize = 1024 * 1024;
const REGEX_VALUE: &str = "_{}_";

#[derive(Debug, Clone)]
pub struct Anime {
    url: String,
    start: u32,
    end: u32,
    path: PathBuf,
}

impl Anime {
    pub fn new(url: &str, start: u32, path: PathBuf) -> Error<Self> {
        let (url, end) = extract(&url)?;

        Ok(Anime {
            url,
            start,
            end,
            path,
        })
    }
    pub fn path(&self) -> Error<String> {
        Ok(format!("{}", self.path.display()))
    }

    pub fn url_episodes(&self) -> Error<Vec<String>> {
        let mut all: Vec<String> = vec![];
        let num_episodes = self.end;

        for i in 1..num_episodes + 1 {
            let num = {
                let mut num = i.to_string();
                if num.len() < 2 {
                    num = format!("0{}", num);
                }
                num
            };

            let url = self.url.replace(REGEX_VALUE, &format!("_{}_", num));
            all.push(url);
        }

        Ok(all)
    }

    pub fn download(url: &str, path: &str) -> Error<()> {
        let r_url = Url::parse(url)?;
        let filename = r_url
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("tmp.bin");

        let client = reqwest::blocking::Client::new();
        let response = client.head(url).send()?;

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

        println!(
            "---\nDownloading {}\nsize = {:?}MB -- {:?}B",
            filename,
            total_size / 1024u64.pow(2),
            total_size,
        );

        for range in PartialRangeIter::new(0, total_size - 1, CHUNK_SIZE)? {
            let mut response = client
                .get(url)
                .header(RANGE, range)
                .send()?
                .error_for_status()?;

            std::io::copy(&mut response, &mut outfile)?;
        }

        Ok(())
    }

    pub async fn _async_download(url: &str) -> Error<()> {
        let r_url = Url::parse(url)?;
        let filename = r_url
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("tmp.bin");

        let client = reqwest::Client::new();
        let response = client.head(url).send().await?;

        let total_size: u64 = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse().ok())
            .unwrap_or(0);

        let mut outfile = tokio::fs::File::create(format!("prova/{}", filename)).await?;

        println!(
            "---\nDownloading {}\nsize = {:?}MB -- {:?}B",
            filename,
            total_size / 1024u64.pow(2),
            total_size,
        );

        for range in PartialRangeIter::new(0, total_size - 1, CHUNK_SIZE)? {
            let response = client
                .get(url)
                .header(RANGE, range)
                .send()
                .await?
                .error_for_status()?;

            let response = response.bytes_stream();
            let response = response
                .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
                .into_async_read();

            let mut response = response.compat();

            // println!("range {:?}MB -- {:?}B", range / 1024u64.pow(2), range);
            tokio::io::copy(&mut response, &mut outfile).await?;
        }

        Ok(())
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
            bail!("invalid buffer_size, give a value greater than zero.");
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

fn extract(url: &str) -> Error<(String, u32)> {
    let re = Regex::new(r"_\d+_")?;
    let end = re.captures(url).unwrap();

    let url = re.replace_all(url, REGEX_VALUE).to_string();
    let end: u32 = end[0].replace("_", "").parse()?;

    Ok((url, end))
}
