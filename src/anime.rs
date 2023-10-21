use anyhow::Result;
use nom::Slice;
use reqwest::header::REFERER;
use reqwest::Client;

#[cfg(feature = "anilist")]
use crate::anilist::AniList;

use crate::range::Range;

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
    pub fn new(name: &str, input: &str, id: Option<u32>, episodes: Option<(u32, u32)>) -> Self {
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
            _ => (input.into(), None),
        };

        AnimeInfo {
            id,
            url,
            episodes,
            num: info_num,
            name: name.to_owned(),
            origin: input.into(),
        }
    }
}

#[derive(Default, Debug)]
pub struct Anime {
    pub episodes: Vec<String>,
    pub info: AnimeInfo,
    pub last_watched: Option<u32>,
    pub start: u32,
}

impl Anime {
    pub fn new(info: &AnimeInfo, last_watched: Option<u32>) -> Self {
        let episodes = match info {
            AnimeInfo {
                url,
                num: Some(InfoNum { alignment, value }),
                episodes: Some((start, end)),
                ..
            } => (*start..=*end)
                .map(|i| gen_url!(url, i + value.checked_sub(1).unwrap_or(*value), alignment))
                .collect(),
            _ => vec![info.url.to_owned()],
        };

        Anime {
            episodes,
            last_watched,
            info: info.to_owned(),
            start: info.num.unwrap_or_default().value,
        }
    }
}

#[cfg(feature = "anilist")]
pub async fn last_watched(client_id: Option<u32>, anime_id: Option<u32>) -> Option<u32> {
    AniList::new(client_id).last_watched(anime_id).await
}

#[cfg(not(feature = "anilist"))]
pub async fn last_watched(_: Option<u32>, _: Option<u32>) -> Option<u32> {
    None
}

pub async fn _find_episodes(
    info: &AnimeInfo,
    referrer: &str,
    range: &Range<u32>,
) -> Result<Vec<String>> {
    let InfoNum { alignment, value } = info.num.unwrap();
    let url = &info.url;
    let client = Client::new();
    let mut err;
    let mut end;
    let mut counter = 2;

    // finds a possible least upper bound
    loop {
        err = counter;
        end = counter / 2;
        match client
            .head(&gen_url!(url, counter, alignment))
            .header(REFERER, referrer)
            .send()
            .await?
            .error_for_status()
        {
            Ok(_) => counter *= 2,
            Err(_) => break,
        }
    }

    // finds the real upper bound with a binary search
    while err != end + 1 {
        counter = (err + end) / 2;
        match client
            .head(&gen_url!(url, counter, alignment))
            .header(REFERER, referrer)
            .send()
            .await?
            .error_for_status()
        {
            Ok(_) => end = counter,
            Err(_) => err = counter,
        }
    }

    // Check if there is a 0 episode
    let start = match range.start() {
        1 => match client
            .head(&gen_url!(url, 0, alignment))
            .header(REFERER, referrer)
            .send()
            .await?
            .error_for_status()
        {
            Ok(_) => 0,
            Err(_) => 1,
        },
        _ => *range.start(),
    };

    Ok((start..=end)
        .map(|i| gen_url!(url, i + value.checked_sub(1).unwrap_or(value), alignment))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_info() {
        let origin = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let res = AnimeInfo::new("Anime Name", origin, None, None);
        assert_eq!(
            res,
            AnimeInfo {
                name: "Anime Name".into(),
                url: url.into(),
                origin: origin.into(),
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
        let res = AnimeInfo::new("Anime Name", origin, Some(14), None);
        assert_eq!(
            res,
            AnimeInfo {
                name: "Anime Name".into(),
                url: url.into(),
                origin: origin.into(),
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
