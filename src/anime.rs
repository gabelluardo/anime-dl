#[cfg(feature = "anilist")]
use crate::parser::{InfoNum, parse_number, parse_url};
use crate::range::Range;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct AnimeInfo {
    pub id: Option<u32>,
    pub name: String,
    pub origin: String,
    pub num: Option<InfoNum>,
    pub url: String,
    pub episodes: Option<Range<u32>>,
    pub last_watched: Option<u32>,
}

impl AnimeInfo {
    pub fn new(name: &str, input: &str, id: Option<u32>, episodes: Option<Range<u32>>) -> Self {
        let info_num = parse_number(input);
        let url = parse_url(input, info_num);

        AnimeInfo {
            id,
            episodes,
            url,
            name: name.into(),
            num: info_num,
            origin: input.into(),
            ..Default::default()
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Anime {
    pub episodes: Vec<String>,
    pub info: AnimeInfo,
    pub start: u32,
}

impl Anime {
    pub fn new(info: &AnimeInfo) -> Self {
        Anime {
            episodes: vec![],
            info: info.to_owned(),
            start: info.num.unwrap_or_default().value,
        }
    }

    pub fn range(&mut self, range: Option<Range<u32>>) {
        self.info.episodes = range
    }

    pub fn expand(&mut self) {
        self.episodes = match &self.info {
            AnimeInfo {
                url,
                num: Some(InfoNum { alignment, value }),
                episodes: Some(Range { start, end }),
                ..
            } => (*start..=*end)
                .map(|i| gen_url!(url, i + value.checked_sub(1).unwrap_or(*value), alignment))
                .collect(),
            _ => vec![self.info.url.to_owned()],
        };
    }

    pub fn select_episodes(&mut self, selection: &[usize]) {
        if let Some(InfoNum { alignment, .. }) = self.info.num {
            self.episodes = selection
                .iter()
                .map(|&i| gen_url!(self.info.url, i as u32, alignment))
                .collect()
        }
    }
}

#[cfg(feature = "anilist")]
pub async fn last_watched(client_id: Option<u32>, anime_id: Option<u32>) -> Option<u32> {
    use crate::anilist::Anilist;

    if let Ok(anilist) = Anilist::new(client_id) {
        return anilist.get_last_watched(anime_id).await;
    }

    None
}

#[cfg(not(feature = "anilist"))]
pub async fn last_watched(_: Option<u32>, _: Option<u32>) -> Option<u32> {
    None
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
                num: Some(InfoNum {
                    value: 15,
                    alignment: 2
                }),
                ..Default::default()
            }
        );

        let origin = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_016_SUB_ITA.mp4";
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let mut res = AnimeInfo::new("Anime Name", origin, Some(14), None);
        res.last_watched = Some(3);
        assert_eq!(
            res,
            AnimeInfo {
                name: "Anime Name".into(),
                url: url.into(),
                origin: origin.into(),
                id: Some(14),
                last_watched: Some(3),
                num: Some(InfoNum {
                    value: 16,
                    alignment: 3
                }),
                ..Default::default()
            }
        );
    }
}
