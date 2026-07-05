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

    #[test_case((1, 5), vec![1, 2, 3, 4, 5]; "iterate 1 to 5")]
    #[test_case((0, 3), vec![0, 1, 2, 3]; "iterate 0 to 3")]
    #[test_case((5, 5), vec![5]; "single value range")]
    #[test_case((3, 1), Vec::<u32>::new(); "start greater than end")]
    #[test]
    fn test_iterator(range: (u32, u32), expected: Vec<u32>) {
        let r = Range::new(range.0, range.1);
        let result: Vec<u32> = r.collect();
        assert_eq!(result, expected);
    }

    #[test_case(1, 0; "default values")]
    #[test]
    fn test_default(start: u32, end: u32) {
        let r = Range::<u32>::default();
        assert_eq!(r.start, start);
        assert_eq!(r.end, end);
    }

    #[test_case((1, 5), (1, 5); "from tuple")]
    #[test_case((0, 0), (0, 0); "from zero tuple")]
    #[test]
    fn test_from_tuple(input: (u32, u32), expected: (u32, u32)) {
        let r: Range<u32> = input.into();
        assert_eq!((r.start, r.end), expected);
    }

    #[test_case("1,5", (1, 5); "comma separated")]
    #[test_case("1.5", (1, 5); "dot separated")]
    #[test_case("(1..5)", (1, 5); "rust style")]
    #[test_case("5", (5, 5); "single value")]
    #[test_case("(0..5)", (0, 5); "rust style no spaces")]
    #[test]
    fn test_from_str_separators(input: &str, expected: (u32, u32)) {
        let r = Range::<u32>::from_str(input).unwrap();
        assert_eq!((r.start, r.end), expected);
    }

    #[test_case("1-5", None, (1, 5); "no end limit")]
    #[test_case("1-", None, (1, 1); "open range no end")]
    #[test]
    fn test_parse_no_end(input: &str, end: Option<u32>, expected: (u32, u32)) {
        let r = Range::<u32>::parse(input, end).unwrap();
        assert_eq!((r.start, r.end), expected);
    }

    #[test_case("" ; "empty string")]
    #[test_case("-" ; "only separator")]
    #[test]
    #[should_panic = "Invalid"]
    fn test_from_str_invalid(s: &str) {
        Range::<u32>::from_str(s).unwrap();
    }
}
