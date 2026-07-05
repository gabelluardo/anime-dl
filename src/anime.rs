use std::str::FromStr;

use derive_more::{Add, Display, From, Into};

use crate::range::Range;

/// Identifies an anime on AniList (media ID).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, From, Display)]
#[display("{_0}")]
pub struct AnimeId(pub u32);

impl From<AnimeId> for i64 {
    fn from(id: AnimeId) -> Self {
        id.0 as i64
    }
}

impl From<i64> for AnimeId {
    fn from(id: i64) -> Self {
        AnimeId(id as u32)
    }
}

impl From<AnimeId> for usize {
    fn from(id: AnimeId) -> Self {
        id.0 as usize
    }
}

impl FromStr for AnimeId {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(Self)
    }
}

/// A zero-based episode number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, From, Display, Add, Into)]
#[display("{_0}")]
pub struct EpisodeId(pub u32);

impl EpisodeId {
    /// Saturating subtraction, like [`u32::checked_sub`].
    pub fn checked_sub(self, rhs: u32) -> Self {
        Self(self.0.checked_sub(rhs).unwrap_or(self.0))
    }
}

impl From<usize> for EpisodeId {
    fn from(n: usize) -> Self {
        Self(n as u32)
    }
}

impl From<EpisodeId> for usize {
    fn from(n: EpisodeId) -> Self {
        n.0 as usize
    }
}

impl From<u8> for EpisodeId {
    fn from(n: u8) -> Self {
        Self(n as u32)
    }
}

impl From<i64> for EpisodeId {
    fn from(n: i64) -> Self {
        Self(n as u32)
    }
}

impl From<EpisodeId> for i64 {
    fn from(n: EpisodeId) -> Self {
        n.0 as i64
    }
}

impl FromStr for EpisodeId {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(Self)
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Anime {
    id: Option<AnimeId>,
    last_watched: Option<EpisodeId>,
    name: String,
    url: String,
    range: Option<Range<EpisodeId>>,
}

impl Anime {
    pub fn new(
        name: impl Into<String>,
        url: impl Into<String>,
        id: Option<AnimeId>,
        range: Option<Range<EpisodeId>>,
    ) -> Self {
        Anime {
            id,
            range,
            name: name.into(),
            url: url.into(),
            last_watched: None,
        }
    }

    pub fn with_last_watched(mut self, last_watched: EpisodeId) -> Self {
        self.last_watched = Some(last_watched);
        self
    }

    pub fn id(&self) -> Option<AnimeId> {
        self.id
    }

    pub fn next_episode(&self) -> EpisodeId {
        let (value, _) = get_episode_number(&self.url).unwrap_or_default();

        value
    }

    pub fn last_watched(&self) -> Option<EpisodeId> {
        self.last_watched
    }

    pub fn last_episode(&self) -> EpisodeId {
        let Some(r) = self.range else {
            return EpisodeId(0);
        };
        r.end
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn range(&self) -> Option<Range<EpisodeId>> {
        self.range
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn select_from_index(&self, start: EpisodeId) -> Vec<String> {
        let Self { url, range, .. } = self;

        match range {
            Some(r) => self.select_from_range(Range::new(start, r.end)),
            None => vec![url.clone()],
        }
    }

    pub fn select_from_range(&self, range: Range<EpisodeId>) -> Vec<String> {
        let Self { url, .. } = self;

        match get_episode_number(url) {
            Some((value, padding)) => {
                let num = value.checked_sub(1);

                range
                    .map(|i| gen_url(url, value, i + num, padding))
                    .collect()
            }
            None => vec![self.url.clone()],
        }
    }

    pub fn select_from_slice(&self, slice: &[EpisodeId]) -> Vec<String> {
        let Self { url, .. } = self;

        match get_episode_number(url) {
            Some((value, padding)) => slice
                .iter()
                .map(|i| gen_url(url, value, *i, padding))
                .collect(),
            None => vec![self.url.clone()],
        }
    }
}

/// Replace url episode number with zero-padded episode number.
pub fn gen_url(url: &str, old: EpisodeId, new: EpisodeId, padding: usize) -> String {
    url.replace(
        &format!("_{old:0fill$}", fill = padding),
        &format!("_{new:0fill$}", fill = padding),
    )
}

/// Extract the episode number and its zero-padding from a URL, if present.
pub fn get_episode_number(url: &str) -> Option<(EpisodeId, usize)> {
    let mut positions =
        url.as_bytes()
            .windows(3)
            .enumerate()
            .filter_map(|(i, window)| match window {
                [b'_', c, cc] if c.is_ascii_digit() && cc.is_ascii_digit() => Some(i),
                [c, cc, b'_'] if c.is_ascii_digit() && cc.is_ascii_digit() => Some(i + 1),
                _ => None,
            });

    let start = positions.next()?;
    let end = positions.next()?;

    if positions.next().is_some() {
        return None;
    }

    let value = url.get(start + 1..end + 1)?;
    let episode: EpisodeId = value.parse().ok()?;
    let padding = value.len();

    Some((episode, padding))
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
        assert_eq!(gen_url(url, old.into(), new.into(), padding), expected);
    }

    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4",
        15, 2;
        "two digit episode"
    )]
    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_016_SUB_ITA.mp4",
        16, 3;
        "three digit episode"
    )]
    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_0017_SUB_ITA.mp4",
        17, 4;
        "four digit episode"
    )]
    #[test]
    fn test_get_episode_number(url: &str, ep: u32, pad: usize) {
        assert_eq!(get_episode_number(url), Some((ep.into(), pad)));
    }

    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4",
        15, 2;
        "two digit episode"
    )]
    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_016_SUB_ITA.mp4",
        16, 3;
        "three digit episode"
    )]
    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_0017_SUB_ITA.mp4",
        17, 4;
        "four digit episode"
    )]
    #[test]
    fn test_remove_episode_number(url: &str, ep: u32, pad: usize) {
        assert_eq!(get_episode_number(url), Some((ep.into(), pad)));
    }

    #[test_case("https://www.domain.tld/file.mp4"; "no episode number")]
    #[test_case("https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4_02_extra"; "multiple matches")]
    #[test_case(""; "empty string")]
    #[test_case("no_digits_here"; "no digits")]
    #[test_case("https://www.domain.tld/_01_02_.mp4"; "two separate episode patterns")]
    #[test]
    fn test_get_episode_number_none(url: &str) {
        assert_eq!(get_episode_number(url), None);
    }

    #[test_case(5, 2, 3; "normal subtraction")]
    #[test_case(5, 10, 5; "underflow saturates")]
    #[test_case(0, 1, 0; "zero minus one")]
    #[test_case(10, 0, 10; "subtract zero")]
    #[test]
    fn test_checked_sub(start: u32, rhs: u32, expected: u32) {
        assert_eq!(EpisodeId(start).checked_sub(rhs), EpisodeId(expected));
    }

    #[test_case("42", 42; "valid id")]
    #[test_case("0", 0; "zero")]
    #[test_case("999999", 999999; "large id")]
    #[test]
    fn test_anime_id_from_str(s: &str, expected: u32) {
        assert_eq!(AnimeId::from_str(s).unwrap(), AnimeId(expected));
    }

    #[test_case("abc"; "non numeric")]
    #[test_case("-1"; "negative")]
    #[test_case("" ; "empty")]
    #[test]
    fn test_anime_id_from_str_err(s: &str) {
        assert!(AnimeId::from_str(s).is_err());
    }

    #[test_case("17", 17; "valid episode")]
    #[test_case("0", 0; "zero episode")]
    #[test]
    fn test_episode_id_from_str(s: &str, expected: u32) {
        assert_eq!(EpisodeId::from_str(s).unwrap(), EpisodeId(expected));
    }

    #[test_case("xyz"; "non numeric")]
    #[test_case("" ; "empty")]
    #[test]
    fn test_episode_id_from_str_err(s: &str) {
        assert!(EpisodeId::from_str(s).is_err());
    }

    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4",
        1, 3,
        vec![
            "https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_02_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_03_SUB_ITA.mp4",
        ];
        "range from episode one"
    )]
    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_05_SUB_ITA.mp4",
        1, 3,
        vec![
            "https://www.domain.tld/AnimeName_Ep_05_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_06_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_07_SUB_ITA.mp4",
        ];
        "range from episode five"
    )]
    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_005_SUB_ITA.mp4",
        1, 3,
        vec![
            "https://www.domain.tld/AnimeName_Ep_005_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_006_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_007_SUB_ITA.mp4",
        ];
        "three digit padding"
    )]
    #[test]
    fn test_select_from_range(url: &str, start: u32, end: u32, expected: Vec<&str>) {
        let anime = Anime::new("Test", url, None, None);
        let range = Range::new(EpisodeId(start), EpisodeId(end));
        let result = anime.select_from_range(range);

        assert_eq!(result, expected);
    }

    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4",
        1, 3,
        vec![
            "https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_02_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_03_SUB_ITA.mp4",
        ];
        "select from index with range"
    )]
    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4",
        1, 1,
        vec!["https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4"];
        "select from index single"
    )]
    #[test]
    fn test_select_from_index(url: &str, start: u32, end: u32, expected: Vec<&str>) {
        let range = Range::new(EpisodeId(start), EpisodeId(end));
        let anime = Anime::new("Test", url, None, Some(range));
        let result = anime.select_from_index(EpisodeId(start));

        assert_eq!(result, expected);
    }

    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4",
        vec![1, 3, 5],
        vec![
            "https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_03_SUB_ITA.mp4",
            "https://www.domain.tld/AnimeName_Ep_05_SUB_ITA.mp4",
        ];
        "select specific episodes"
    )]
    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4",
        vec![1],
        vec!["https://www.domain.tld/AnimeName_Ep_01_SUB_ITA.mp4"];
        "select single episode"
    )]
    #[test]
    fn test_select_from_slice(url: &str, episodes: Vec<u32>, expected: Vec<&str>) {
        let anime = Anime::new("Test", url, None, None);
        let slice: Vec<EpisodeId> = episodes.into_iter().map(EpisodeId).collect();
        let result = anime.select_from_slice(&slice);

        assert_eq!(result, expected);
    }

    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_15_SUB_ITA.mp4",
        15;
        "extract episode number"
    )]
    #[test_case(
        "https://www.domain.tld/file.mp4",
        0;
        "no episode number defaults to zero"
    )]
    #[test]
    fn test_next_episode(url: &str, expected: u32) {
        let anime = Anime::new("Test", url, None, None);
        assert_eq!(anime.next_episode(), EpisodeId(expected));
    }

    #[test_case(None, 0; "no range returns zero")]
    #[test_case(Some((1, 12)), 12; "range end is twelve")]
    #[test_case(Some((5, 100)), 100; "range end is one hundred")]
    #[test]
    fn test_last_episode(range: Option<(u32, u32)>, expected: u32) {
        let range = range.map(|(s, e)| Range::new(EpisodeId(s), EpisodeId(e)));
        let anime = Anime::new("Test", "https://domain.tld/file.mp4", None, range);
        assert_eq!(anime.last_episode(), EpisodeId(expected));
    }

    #[test_case(None, None; "no last watched")]
    #[test_case(Some(EpisodeId(5)), Some(EpisodeId(5)); "with last watched")]
    #[test_case(Some(EpisodeId(0)), Some(EpisodeId(0)); "zero last watched")]
    #[test]
    fn test_with_last_watched(initial: Option<EpisodeId>, expected: Option<EpisodeId>) {
        let anime = Anime::new("Test", "https://domain.tld/file.mp4", None, None);
        assert_eq!(anime.last_watched(), None);

        let anime = match initial {
            Some(ep) => anime.with_last_watched(ep),
            None => anime,
        };
        assert_eq!(anime.last_watched(), expected);
    }

    #[test_case(
        Some(AnimeId(42)), "Name", "https://domain.tld/file.mp4",
        Some((1, 12));
        "all set"
    )]
    #[test_case(
        None, "Test", "https://domain.tld/other.mp4",
        None;
        "all none"
    )]
    #[test]
    fn test_anime_accessors(id: Option<AnimeId>, name: &str, url: &str, range: Option<(u32, u32)>) {
        let range = range.map(|(s, e)| Range::new(EpisodeId(s), EpisodeId(e)));
        let anime = Anime::new(name, url, id, range);

        assert_eq!(anime.id(), id);
        assert_eq!(anime.name(), name);
        assert_eq!(anime.url(), url);
        assert_eq!(anime.range(), range);
    }

    #[test_case(42, 42; "anime id to i64")]
    #[test_case(0, 0; "zero anime id")]
    #[test]
    fn test_anime_id_to_i64(input: u32, expected: i64) {
        assert_eq!(i64::from(AnimeId(input)), expected);
    }

    #[test_case(42, 42; "i64 to anime id")]
    #[test_case(0, 0; "zero i64")]
    #[test]
    fn test_i64_to_anime_id(input: i64, expected: u32) {
        assert_eq!(AnimeId::from(input), AnimeId(expected));
    }

    #[test_case(42, 42; "anime id to usize")]
    #[test_case(0, 0; "zero to usize")]
    #[test]
    fn test_anime_id_to_usize(input: u32, expected: usize) {
        assert_eq!(usize::from(AnimeId(input)), expected);
    }

    #[test_case(5, 5; "episode id to usize")]
    #[test_case(0, 0; "zero episode to usize")]
    #[test]
    fn test_episode_id_to_usize(input: u32, expected: usize) {
        assert_eq!(usize::from(EpisodeId(input)), expected);
    }

    #[test_case(5, 5; "usize to episode id")]
    #[test_case(0, 0; "zero usize")]
    #[test]
    fn test_usize_to_episode_id(input: usize, expected: u32) {
        assert_eq!(EpisodeId::from(input), EpisodeId(expected));
    }

    #[test_case(5, 5; "u8 to episode id")]
    #[test_case(0, 0; "zero u8")]
    #[test]
    fn test_u8_to_episode_id(input: u8, expected: u32) {
        assert_eq!(EpisodeId::from(input), EpisodeId(expected));
    }

    #[test_case(5, 5; "i64 to episode id")]
    #[test_case(0, 0; "zero i64")]
    #[test]
    fn test_i64_to_episode_id(input: i64, expected: u32) {
        assert_eq!(EpisodeId::from(input), EpisodeId(expected));
    }

    #[test_case(5, 5; "episode id to i64")]
    #[test_case(0, 0; "zero episode to i64")]
    #[test]
    fn test_episode_id_to_i64(input: u32, expected: i64) {
        assert_eq!(i64::from(EpisodeId(input)), expected);
    }
}
