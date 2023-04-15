use std::ops::{Deref, RangeInclusive as OpsRange};
use std::str::FromStr;

use anyhow::{bail, Result};

use crate::errors::UserError;

#[derive(Debug, Clone)]
pub struct Range<T>(OpsRange<T>);

impl<T> Range<T>
where
    T: Copy + Clone + FromStr + Ord,
{
    pub fn new(start: T, end: T) -> Self {
        Self(start..=end)
    }

    pub fn expand(&self) -> OpsRange<T> {
        *self.start()..=*self.end()
    }

    pub fn parse(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        Self::from_str(s)
    }

    pub fn parse_and_fill(s: &str, end: T) -> Result<Self, <Self as FromStr>::Err> {
        Self::parse(s).map(|r| {
            if r.end().gt(&end) || r.end().eq(r.start()) {
                Self::new(*r.start(), end)
            } else {
                r
            }
        })
    }
}

impl Default for Range<u32> {
    fn default() -> Self {
        Self(1..=1)
    }
}

impl Default for &Range<u32> {
    fn default() -> Self {
        &Range(1..=1)
    }
}

impl<T> Deref for Range<T> {
    type Target = OpsRange<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromStr for Range<T>
where
    T: Copy + Clone + FromStr + Ord,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let range_str = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(&[',', '-', '.'][..])
            .collect::<Vec<_>>();
        match (range_str.first(), range_str.last()) {
            (Some(&f), Some(&l)) => match (f.parse::<T>(), l.parse::<T>()) {
                (Ok(s), Ok(e)) => Ok(Self(s..=e)),
                (Ok(s), Err(_)) => Ok(Self(s..=s)),
                _ => bail!(UserError::InvalidRange),
            },
            _ => bail!(UserError::InvalidRange),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let range1 = Range::new(0, 1);
        let (start, end) = (*range1.start(), *range1.end());
        assert_eq!(start, 0);
        assert_eq!(end, 1);

        let range2 = Range::<i32>::from_str("(0..1)").unwrap();
        assert_eq!(*range2.start(), 0);
        assert_eq!(*range2.end(), 1);

        assert!(range1.expand().eq(range2.expand()));

        let range3 = Range::default();
        assert_eq!((*range3.start(), *range3.end()), (1, 1));

        let range4 = Range::<i32>::from_str("1-5").unwrap();
        assert_eq!((*range4.start(), *range4.end()), (1, 5));

        let range5 = Range::<i32>::from_str("1-").unwrap();
        assert_eq!((*range5.start(), *range5.end()), (1, 1));

        let range6 = Range::<i32>::parse_and_fill("1-", 6).unwrap();
        assert_eq!((*range6.start(), *range6.end()), (1, 6));
    }

    #[test]
    #[should_panic]
    fn test_wrong_range() {
        Range::<i32>::from_str("-").unwrap();
    }
}
