use crate::range::Range;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Anime {
    id: Option<u32>,
    last_watched: Option<i64>,
    name: String,
    url: String,
    range: Option<Range<u32>>,
}

impl Anime {
    pub fn new(name: String, url: String, id: Option<u32>, range: Option<Range<u32>>) -> Self {
        Anime {
            id,
            range,
            name,
            url,
            last_watched: None,
        }
    }

    pub fn with_last_watched(mut self, last_watched: i64) -> Self {
        self.last_watched = Some(last_watched);
        self
    }

    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn next_episode(&self) -> u32 {
        let (value, _) = get_episode_number(&self.url).unwrap_or_default();

        value
    }

    pub fn last_watched(&self) -> Option<i64> {
        self.last_watched
    }

    pub fn last_episode(&self) -> u32 {
        self.range.unwrap_or_default().end
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn range(&self) -> Option<Range<u32>> {
        self.range
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn select_from_index(&self, start: usize) -> Vec<String> {
        let Self { url, range, .. } = self;

        match range {
            Some(r) => self.select_from_range(Range::new(start as u32, r.end)),
            None => vec![url.clone()],
        }
    }

    pub fn select_from_range(&self, range: Range<u32>) -> Vec<String> {
        let Self { url, .. } = self;

        match get_episode_number(url) {
            Some((value, padding)) => {
                let num = value.checked_sub(1).unwrap_or(value);

                range
                    .map(|i| gen_url(url, value, i + num, padding))
                    .collect()
            }
            None => vec![self.url.clone()],
        }
    }

    pub fn select_from_slice(&self, slice: &[usize]) -> Vec<String> {
        let Self { url, .. } = self;

        match get_episode_number(url) {
            Some((value, padding)) => slice
                .iter()
                .map(|&i| gen_url(url, value, i as u32, padding))
                .collect(),
            None => vec![self.url.clone()],
        }
    }
}

/// Replace url episode namber with zero-padded episode number.
pub fn gen_url(url: &str, old: u32, new: u32, padding: usize) -> String {
    url.replace(
        &format!("_{old:0fill$}", fill = padding),
        &format!("_{new:0fill$}", fill = padding),
    )
}

/// Extract the episode number and its zero-padding from a URL, if present.
pub fn get_episode_number(url: &str) -> Option<(u32, usize)> {
    let chars: Vec<_> = url.chars().collect();
    let positions: Vec<_> = chars
        .array_windows()
        .enumerate()
        .filter_map(|(i, window)| match window {
            ['_', c, cc] if c.is_ascii_digit() && cc.is_ascii_digit() => Some(i),
            [c, cc, '_'] if c.is_ascii_digit() && cc.is_ascii_digit() => Some(i + 1),
            _ => None,
        })
        .collect();

    match positions.as_slice() {
        [start, end] => {
            let str = &url[*start + 1..*end + 1];

            let value: u32 = str.parse().ok()?;
            let padding = str.len();

            Some((value, padding))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case("https://robe_01_.tld", 1, 42, 2, "https://robe_42_.tld"; "two digits to larger value")]
    #[test_case("https://robe_01_.tld", 1, 14, 2, "https://robe_14_.tld"; "two digits to other value")]
    #[test_case("https://robe_42_.tld", 42, 1, 2, "https://robe_01_.tld"; "two digits with leading zero")]
    #[test_case("https://robe_42_.tld", 42, 14, 2, "https://robe_14_.tld"; "two digits replacement")]
    #[test_case("https://robe_042_.tld", 42, 1, 3, "https://robe_001_.tld"; "three digits with leading zeros")]
    #[test_case("https://robe_042_.tld", 42, 14, 3, "https://robe_014_.tld"; "three digits replacement")]
    #[test_case("https://robe_042_.tld", 42, 1400, 3, "https://robe_1400_.tld"; "replacement longer than padding")]
    #[test]
    fn test_gen_url(url: &str, old: u32, new: u32, padding: usize, expected: &str) {
        assert_eq!(gen_url(url, old, new, padding), expected);
    }

    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4",
        Some((15, 2));
        "two digit episode"
    )]
    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_016_SUB_ITA.mp4",
        Some((16, 3));
        "three digit episode"
    )]
    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_0017_SUB_ITA.mp4",
        Some((17, 4));
        "four digit episode"
    )]
    #[test]
    fn test_get_episode_number(url: &str, expected: Option<(u32, usize)>) {
        assert_eq!(get_episode_number(url), expected);
    }

    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4",
        Some((15, 2));
        "two digit episode"
    )]
    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_016_SUB_ITA.mp4",
        Some((16, 3));
        "three digit episode"
    )]
    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_0017_SUB_ITA.mp4",
        Some((17, 4));
        "four digit episode"
    )]
    #[test]
    fn test_remove_episode_number(url: &str, expected: Option<(u32, usize)>) {
        assert_eq!(get_episode_number(url), expected);
    }
}
