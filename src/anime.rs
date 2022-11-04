use std::path::{Path, PathBuf};

use nom::Slice;
use reqwest::header::REFERER;
use reqwest::Client;
use tokio::fs;

#[cfg(feature = "anilist")]
use crate::anilist::AniList;
use crate::errors::{Error, Result};
use crate::utils::{self, tui, Range};

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct InfoNum {
    pub value: u32,
    pub alignment: usize,
}
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct AnimeInfo {
    pub id: Option<u32>,
    pub name: String,
    pub url: String,
    pub origin: String,
    pub num: Option<InfoNum>,
}

impl AnimeInfo {
    pub fn new(input: &str, id: Option<u32>) -> Result<Self> {
        let name = utils::parse_name(input)?;

        // find episode number position in input
        let (mut opt_start, mut opt_end) = (None, None);
        for (i, c) in input.char_indices() {
            if let Some(next) = input.chars().nth(i + 1) {
                if c == '_' && next.is_ascii_digit() {
                    opt_start = Some(i);
                } else if c.is_ascii_digit() && next == '_' {
                    opt_end = Some(i);
                }
            }
        }

        let (url, info_num) = match (opt_start, opt_end) {
            (Some(start_pos), Some(end_pos)) => {
                let sub_str = input
                    .slice(start_pos..end_pos + 1)
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>();

                let url = input.replace(&sub_str, "{}");
                let info_num = sub_str.parse::<u32>().ok().map(|value| InfoNum {
                    value,
                    alignment: sub_str.len(),
                });

                (url, info_num)
            }
            _ => (input.to_string(), None),
        };

        Ok(AnimeInfo {
            id,
            name,
            url,
            num: info_num,
            origin: input.to_string(),
        })
    }
}

#[derive(Default, Debug)]
pub struct AnimeBuilder {
    auto: bool,
    client_id: Option<u32>,
    path: PathBuf,
    range: Range<u32>,
    referrer: String,
    info: AnimeInfo,
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

    pub fn info(mut self, info: &AnimeInfo) -> Self {
        self.info = info.to_owned();
        self
    }

    pub fn range(mut self, range: &Range<u32>) -> Self {
        self.range = range.to_owned();
        self
    }

    pub fn path(mut self, path: &Path) -> Self {
        self.path = path.to_owned();
        self
    }

    pub fn referer(mut self, referer: &str) -> Self {
        self.referrer = referer.to_string();
        self
    }

    pub async fn build(mut self) -> Result<Anime> {
        let episodes = match self.info.num {
            Some(_) => self.episodes().await?,
            _ => vec![self.info.url.clone()],
        };

        let last_viewed = self.last_viewed().await;

        Ok(Anime {
            episodes,
            last_viewed,
            info: self.info,
            path: self.path,
        })
    }

    async fn episodes(&mut self) -> Result<Vec<String>> {
        let url = &self.info.url;
        let InfoNum { alignment, .. } = self.info.num.as_ref().unwrap();

        if self.auto {
            let client = Client::new();
            let mut err;
            let mut last;
            let mut counter = 2;

            // finds a possible least upper bound
            loop {
                err = counter;
                last = counter / 2;

                match client
                    .head(&gen_url!(url, counter, alignment))
                    .header(REFERER, &self.referrer)
                    .send()
                    .await?
                    .error_for_status()
                {
                    Ok(_) => counter *= 2,
                    Err(_) => break,
                }
            }

            // finds the real upper bound with a binary search
            while err != last + 1 {
                counter = (err + last) / 2;

                match client
                    .head(&gen_url!(url, counter, alignment))
                    .header(REFERER, &self.referrer)
                    .send()
                    .await?
                    .error_for_status()
                {
                    Ok(_) => last = counter,
                    Err(_) => err = counter,
                }
            }

            // Check if there is a 0 episode
            let first = match self.range.start() {
                1 => match client
                    .head(&gen_url!(url, 0, alignment))
                    .header(REFERER, &self.referrer)
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
            bail!(Error::Download(String::new()))
        }

        let episodes = self
            .range
            .expand()
            .map(|i| gen_url!(url, i, alignment))
            .collect::<Vec<_>>();

        Ok(episodes)
    }

    #[cfg(feature = "anilist")]
    async fn last_viewed(&self) -> Option<u32> {
        let anilist = AniList::new(self.client_id).unwrap_or_default();

        anilist.last_viewed(self.info.id).await.unwrap_or_default()
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
    pub info: AnimeInfo,
}

impl Anime {
    pub fn builder() -> AnimeBuilder {
        AnimeBuilder::default()
    }

    pub fn choices(&self) -> Vec<tui::Choice> {
        let mut choices = vec![];
        let mut start_range = 0;
        let mut align = 0;

        for (i, ep) in self.episodes.iter().enumerate() {
            let mut msg = self.info.name.to_string();

            // find first episode number
            if start_range == 0 {
                if let Ok(info) = AnimeInfo::new(ep, None) {
                    if let Some(InfoNum { value, alignment }) = info.num {
                        start_range = value;
                        align = alignment;
                    }
                }
            }

            let num = start_range + i as u32;

            msg.push_str(&format!(" - ep {:0fill$}", num, fill = align));

            if Some(num) <= self.last_viewed {
                msg.push_str(" ✔")
            }

            choices.push(tui::Choice::new(ep.to_string(), msg))
        }

        choices
    }
}

pub struct FileDest {
    pub size: u64,
    pub root: PathBuf,
    pub file: PathBuf,
    pub overwrite: bool,
}

type FileProps<'a> = (&'a Path, &'a str, bool);

impl FileDest {
    pub async fn new(props: FileProps<'_>) -> Result<Self> {
        let (root, filename, overwrite) = props;

        let root = root.to_owned();
        let overwrite = overwrite.to_owned();

        if !root.exists() {
            fs::create_dir_all(&root).await?;
        }

        let mut file = root.clone();
        file.push(filename);

        let size = match file.exists() && !overwrite {
            true => fs::File::open(&file).await?.metadata().await?.len(),
            false => 0,
        };

        Ok(Self {
            size,
            root,
            file,
            overwrite,
        })
    }

    pub async fn open(&self) -> Result<fs::File> {
        fs::OpenOptions::new()
            .append(!self.overwrite)
            .truncate(self.overwrite)
            .write(self.overwrite)
            .create(true)
            .open(&self.file)
            .await
            .map_err(|_| Error::FsOpen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_info() {
        let input = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let res = AnimeInfo::new(input, None).unwrap();
        let num = res.num.unwrap();

        assert_eq!(res.name, "Anime Name");
        assert_eq!(res.url, url);
        assert_eq!(res.origin, input);
        assert_eq!(res.id, None);
        assert_eq!(num.value, 15);
        assert_eq!(num.alignment, 2);

        let input = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_016_SUB_ITA.mp4";
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let res = AnimeInfo::new(input, Some(14)).unwrap();
        let num = res.num.unwrap();

        assert_eq!(res.name, "Anime Name");
        assert_eq!(res.url, url);
        assert_eq!(res.origin, input);
        assert_eq!(res.id, Some(14));
        assert_eq!(num.value, 16);
        assert_eq!(num.alignment, 3);
    }
}
