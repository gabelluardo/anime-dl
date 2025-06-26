#[cfg(feature = "anilist")]
use crate::parser::{InfoNum, parse_number, parse_url};
use crate::range::Range;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Anime {
    pub id: Option<u32>,
    pub last_watched: Option<u32>,
    pub name: String,
    pub num: Option<InfoNum>,
    pub origin: String,
    pub range: Option<Range<u32>>,
    pub start: u32,
    pub url: String,
}

impl Anime {
    pub fn new(name: &str, input: &str, id: Option<u32>, range: Option<Range<u32>>) -> Self {
        let num = parse_number(input);
        let url = parse_url(input, num);

        Anime {
            id,
            num,
            range,
            url,
            name: name.into(),
            origin: input.into(),
            start: num.unwrap_or_default().value,
            ..Default::default()
        }
    }
    pub fn select_from_index(&self, start: u32) -> Vec<String> {
        if let Some(Range { end, .. }) = self.range {
            return self.select_from_range(Range::new(start, end));
        }

        vec![self.url.clone()]
    }

    pub fn select_from_range(&self, range: Range<u32>) -> Vec<String> {
        if let Some(num) = self.num {
            let value = num.value.checked_sub(1).unwrap_or(num.value);

            return range
                .map(|i| gen_url!(self.url, i + value, num.alignment))
                .collect();
        }

        vec![self.url.clone()]
    }

    pub fn select_from_slice(&self, slice: &[usize]) -> Vec<String> {
        if let Some(num) = self.num {
            return slice
                .iter()
                .map(|&i| gen_url!(self.url, i as u32, num.alignment))
                .collect();
        }

        vec![self.url.clone()]
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
        let res = Anime::new("Anime Name", origin, None, None);
        assert_eq!(
            res,
            Anime {
                name: "Anime Name".into(),
                url: url.into(),
                origin: origin.into(),
                num: Some(InfoNum {
                    value: 15,
                    alignment: 2
                }),
                start: 15,
                ..Default::default()
            }
        );

        let origin = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_016_SUB_ITA.mp4";
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let mut res = Anime::new("Anime Name", origin, Some(14), None);
        res.last_watched = Some(3);
        assert_eq!(
            res,
            Anime {
                name: "Anime Name".into(),
                url: url.into(),
                origin: origin.into(),
                id: Some(14),
                last_watched: Some(3),
                num: Some(InfoNum {
                    value: 16,
                    alignment: 3
                }),
                start: 16,
                ..Default::default()
            }
        );
    }
}
