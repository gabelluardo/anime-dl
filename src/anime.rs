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
}
