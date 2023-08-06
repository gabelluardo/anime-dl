use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use nom::Slice;
use reqwest::header::REFERER;
use reqwest::Client;

#[cfg(feature = "anilist")]
use crate::anilist::AniList;
use crate::range::Range;
use crate::tui::Choice;
use crate::utils;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct InfoNum {
    pub value: u32,
    pub alignment: usize,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct AnimeInfo {
    pub id: Option<u32>,
    pub name: String,
    pub origin: String,
    pub num: Option<InfoNum>,
    pub url: String,
    pub episodes: Option<(u32, u32)>,
}

impl AnimeInfo {
    pub fn new(input: &str, id: Option<u32>, episodes: Option<(u32, u32)>) -> Self {
        let name = to_title_case!(utils::parse_name(input).unwrap());

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

        AnimeInfo {
            id,
            name,
            url,
            episodes,
            num: info_num,
            origin: input.to_string(),
        }
    }
}

#[derive(Default, Debug)]
pub struct AnimeBuilder {
    auto: bool,
    client_id: Option<u32>,
    info: AnimeInfo,
    path: PathBuf,
    range: Range<u32>,
    referrer: String,
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
        let episodes = if self.info.num.is_some() {
            self.episodes().await?
        } else {
            vec![self.info.url.clone()]
        };

        let last_watched = self.last_watched().await;

        Ok(Anime {
            episodes,
            last_watched,
            info: self.info,
            path: self.path,
            range: self.range,
        })
    }

    async fn episodes(&mut self) -> Result<Vec<String>> {
        let url = &self.info.url;
        let InfoNum { alignment, value } = self.info.num.unwrap();

        if let Some((start, end)) = self.info.episodes {
            self.range = Range::new(start, end);
        } else if self.auto {
            self.range = self.fill_range(url, alignment).await?;
        }

        if self.range.is_empty() {
            bail!("Unable to download")
        }

        // for when the range starts with episode 0
        let first = if value > 0 { value - 1 } else { value };

        let episodes = self
            .range
            .expand()
            .map(|i| gen_url!(url, i + first, alignment))
            .collect::<Vec<_>>();
        Ok(episodes)
    }

    async fn fill_range(&self, url: &str, alignment: usize) -> Result<Range<u32>> {
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
        Ok(Range::new(first, last))
    }

    #[cfg(feature = "anilist")]
    async fn last_watched(&self) -> Option<u32> {
        AniList::new(self.client_id)
            .last_watched(self.info.id)
            .await
    }

    #[cfg(not(feature = "anilist"))]
    async fn last_watched(&self) -> Option<u32> {
        None
    }
}

#[derive(Default, Debug)]
pub struct Anime {
    pub last_watched: Option<u32>,
    pub episodes: Vec<String>,
    pub path: PathBuf,
    pub info: AnimeInfo,
    pub range: Range<u32>,
}

impl Anime {
    pub fn builder() -> AnimeBuilder {
        AnimeBuilder::default()
    }

    pub fn choices(&self) -> Vec<Choice> {
        let mut choices = vec![];
        for (i, ep) in self.episodes.iter().enumerate() {
            let num = self.range.start() + i as u32;
            let watched = Some(num) <= self.last_watched;
            let name = self.info.name.to_string();
            choices.push(Choice::new(ep, &name, Some(watched)))
        }
        choices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_info() {
        let origin = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let res = AnimeInfo::new(origin, None, None);
        assert_eq!(
            res,
            AnimeInfo {
                name: "Anime Name".to_string(),
                url: url.to_string(),
                origin: origin.to_string(),
                id: None,
                episodes: None,
                num: Some(InfoNum {
                    value: 15,
                    alignment: 2
                })
            }
        );

        let origin = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_016_SUB_ITA.mp4";
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let res = AnimeInfo::new(origin, Some(14), None);
        assert_eq!(
            res,
            AnimeInfo {
                name: "Anime Name".to_string(),
                url: url.to_string(),
                origin: origin.to_string(),
                id: Some(14),
                episodes: None,
                num: Some(InfoNum {
                    value: 16,
                    alignment: 3
                })
            }
        );
    }
}
