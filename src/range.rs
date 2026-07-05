use std::str::FromStr;

use anyhow::Result;

use crate::error::RangeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}

impl<T> Iterator for Range<T>
where
    T: Copy + PartialOrd + std::ops::Add<Output = T> + From<u8>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            return None;
        }

        let current = self.start;
        self.start = self.start + T::from(1);

        Some(current)
    }
}

impl<T> Range<T>
where
    T: Copy + FromStr + Ord,
{
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    pub fn parse(s: &str, end: Option<T>) -> Result<Self, <Self as FromStr>::Err> {
        let result = Self::from_str(s)?;
        let is_open = s.trim_end().ends_with('-');

        if let Some(end) = end
            && (is_open || result.end > end)
        {
            return Ok(Self::new(result.start, end));
        }

        Ok(result)
    }
}

impl Default for Range<u32> {
    fn default() -> Self {
        Self { start: 1, end: 0 }
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
    type Err = RangeError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let mut values = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(&[',', '-', '.'])
            .filter_map(|c| c.parse::<T>().ok());

        let start = values.next().ok_or(RangeError::Invalid)?;
        let mut end = start;
        for value in values {
            end = value;
        }

        Ok(Self::new(start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case("(0..5)", (0, 5); "rust style range")]
    #[test_case("1-5", (1, 5); "hyphen range")]
    #[test_case("1-", (1, 1); "open range without parse expansion")]
    #[test]
    fn test_from_str(input: &str, expected: (u32, u32)) {
        let range = Range::<u32>::from_str(input).unwrap();
        assert_eq!((range.start, range.end), expected);
    }

    #[test_case((0, 1), "(0..1)"; "rust style range")]
    #[test_case((1, 5), "1-5"; "hyphen range")]
    #[test_case((4, 8), "(4..8)"; "larger rust style range")]
    #[test_case((6, 6), "6"; "single value range")]
    #[test]
    fn test_expand(expected: (u32, u32), input: &str) {
        let range1 = Range::from(expected);
        let range2 = Range::<u32>::from_str(input).unwrap();

        assert!(range1.eq(range2));
    }

    #[test_case("1-", Some(6), (1, 6); "expand open range from one")]
    #[test_case("4-", Some(6), (4, 6); "expand open range from four")]
    #[test_case("4-8", Some(12), (4, 8); "keep bounded range within limit")]
    #[test_case("4-8", Some(6), (4, 6); "clamp bounded range to limit")]
    #[test_case("3", Some(6), (3, 3); "single value not expanded")]
    #[test_case("3", Some(10), (3, 3); "single value not expanded larger")]
    #[test]
    fn test_parse(input: &str, end: Option<u32>, expected: (u32, u32)) {
        let range = Range::parse(input, end).unwrap();
        assert_eq!((range.start, range.end), expected);
    }

    #[test]
    #[should_panic = "Invalid"]
    fn test_wrong_range() {
        Range::<i32>::from_str("-").unwrap();
    }
}
