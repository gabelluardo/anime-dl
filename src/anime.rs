use crate::range::Range;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Anime {
    pub id: Option<u32>,
    pub last_watched: Option<i64>,
    pub name: String,
    pub num: Option<InfoNum>,
    pub origin: String,
    pub range: Option<Range<u32>>,
    pub start: u32,
    pub url: String,
}

impl Anime {
    pub fn new(name: &str, input: &str, id: Option<u32>, range: Option<Range<u32>>) -> Self {
        let num = get_episode_number(input);
        let url = remove_episode_number(input, num);
        let start = num.unwrap_or_default().value;

        Anime {
            id,
            num,
            range,
            start,
            url,
            last_watched: None,
            name: name.into(),
            origin: input.into(),
        }
    }

    pub fn select_from_index(&self, start: u32) -> Vec<String> {
        let Self { url, range, .. } = self;

        match range {
            Some(r) => self.select_from_range(Range::new(start, r.end)),
            None => vec![url.clone()],
        }
    }

    pub fn select_from_range(&self, range: Range<u32>) -> Vec<String> {
        let Self { url, num, .. } = self;

        match num {
            Some(InfoNum { value, alignment }) => {
                let value = value.checked_sub(1).unwrap_or(*value);
                range.map(|i| gen_url(url, i + value, *alignment)).collect()
            }
            None => vec![url.clone()],
        }
    }

    pub fn select_from_slice(&self, slice: &[usize]) -> Vec<String> {
        let Self { url, num, .. } = self;

        match num {
            Some(InfoNum { alignment, .. }) => slice
                .iter()
                .map(|&i| gen_url(&self.url, i as u32, *alignment))
                .collect(),
            None => vec![url.clone()],
        }
    }
}

/// Fill url placeholder with zero-padded episode number.
pub fn gen_url(url: &str, num: u32, alignment: usize) -> String {
    url.replace("_{}", &format!("_{:0fill$}", num, fill = alignment))
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct InfoNum {
    pub value: u32,
    pub alignment: usize,
}

/// Replace the detected episode number in a URL with a `{}` placeholder.
pub fn remove_episode_number(input: &str, num: Option<InfoNum>) -> String {
    match num {
        Some(InfoNum { value, alignment }) => {
            let num = format!("{:0fill$}", value, fill = alignment);
            input.replace(&num, "{}")
        }
        None => input.into(),
    }
}

/// Extract the episode number and its zero-padding from a URL, if present.
pub fn get_episode_number(input: &str) -> Option<InfoNum> {
    let chars: Vec<_> = input.chars().collect();
    let positions: Vec<_> = chars
        .windows(2)
        .enumerate()
        .filter_map(|(i, window)| match window {
            ['_', c] if c.is_ascii_digit() => Some(i),
            [c, '_'] if c.is_ascii_digit() => Some(i),
            _ => None,
        })
        .collect();

    match positions.as_slice() {
        [start, end] => {
            let str = &input[*start + 1..*end + 1];

            let value = str.parse::<u32>().ok()?;
            let alignment = str.len();

            Some(InfoNum { value, alignment })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_url() {
        let url = "https://robe_{}_.tld";

        assert_eq!(gen_url(url, 1, 2), "https://robe_01_.tld");
        assert_eq!(gen_url(url, 14, 2), "https://robe_14_.tld");
        assert_eq!(gen_url(url, 1400, 2), "https://robe_1400_.tld");

        assert_eq!(gen_url(url, 1, 3), "https://robe_001_.tld");
        assert_eq!(gen_url(url, 14, 3), "https://robe_014_.tld");
        assert_eq!(gen_url(url, 1400, 3), "https://robe_1400_.tld");
    }

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
