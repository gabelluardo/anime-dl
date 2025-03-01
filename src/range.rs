use std::ops::RangeInclusive;
use std::str::FromStr;

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}

impl<T> Range<T>
where
    T: Copy + FromStr + Ord,
{
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    pub fn expand(&self) -> RangeInclusive<T> {
        (*self).into()
    }

    pub fn parse(s: &str, end: Option<T>) -> Result<Self, <Self as FromStr>::Err> {
        match (Self::from_str(s), end) {
            (Ok(r), Some(end)) if r.end.gt(&end) || r.end.eq(&r.start) => {
                Ok(Self::new(r.start, end))
            }
            (res, _) => res,
        }
    }
}

impl Default for Range<u32> {
    fn default() -> Self {
        Self { start: 1, end: 0 }
    }
}

impl<T> From<Range<T>> for RangeInclusive<T> {
    fn from(val: Range<T>) -> Self {
        val.start..=val.end
    }
}

impl<T> From<(T, T)> for Range<T>
where
    T: Copy + FromStr + Ord,
{
    fn from((start, end): (T, T)) -> Self {
        Self::new(start, end)
    }
}

impl<T> FromStr for Range<T>
where
    T: Copy + FromStr + Ord,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let range_str = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(&[',', '-', '.'])
            .filter_map(|c| c.parse::<T>().ok())
            .collect::<Vec<_>>();
        let range = match range_str[..] {
            [start, .., end] => Self::new(start, end),
            [start] => Self::new(start, start),
            _ => bail!("Invalid range"),
        };

        Ok(range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let range = Range::<u32>::from_str("(0..5)").unwrap();
        assert_eq!((range.start, range.end), (0, 5));

        let range = Range::from_str("1-5").unwrap();
        assert_eq!((range.start, range.end), (1, 5));

        let range = Range::from_str("1-").unwrap();
        assert_eq!((range.start, range.end), (1, 1));
    }

    #[test]
    fn test_expand() {
        let range1 = Range::new(0, 1);
        let range2 = Range::<u32>::from_str("(0..1)").unwrap();

        assert!(range1.expand().eq(range2.expand()));
    }

    #[test]
    fn test_parse() {
        let range = Range::parse("1-", Some(6)).unwrap();
        assert_eq!((range.start, range.end), (1, 6));

        let range = Range::parse("4-", Some(6)).unwrap();
        assert_eq!((range.start, range.end), (4, 6));

        let range = Range::parse("4-8", Some(12)).unwrap();
        assert_eq!((range.start, range.end), (4, 8));

        let range = Range::parse("4-8", Some(6)).unwrap();
        assert_eq!((range.start, range.end), (4, 6));
    }

    #[test]
    #[should_panic]
    fn test_wrong_range() {
        Range::<i32>::from_str("-").unwrap();
    }
}
