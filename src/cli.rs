use anyhow::{bail, Result};
use structopt::{clap::arg_enum, StructOpt};

use std::iter::FromIterator;
use std::ops::{self, Deref};
use std::path::PathBuf;
use std::str::FromStr;

arg_enum! {
    #[derive(Debug, Copy, Clone)]
    pub enum Site {
        AW,
        AS,
    }
}

#[derive(Debug, Clone)]
pub struct Range<T>(ops::Range<T>);

impl<'a, T> Range<T>
where
    T: Copy + Clone + FromStr + Ord,
{
    pub fn new(start: T, end: T) -> Self {
        Self(start..end)
    }

    pub fn range(&self) -> ops::Range<T> {
        self.start..self.end
    }

    pub fn parse(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        Self::from_str(s)
    }

    pub fn parse_and_fill(s: &str, end: T) -> Result<Self, <Self as FromStr>::Err> {
        Self::parse(s).map(|r| {
            if r.end.gt(&end) || r.end.eq(&r.start) {
                Self::new(r.start, end)
            } else {
                r
            }
        })
    }
}

impl Default for Range<u32> {
    fn default() -> Self {
        Self(1..0)
    }
}

impl<T> Deref for Range<T> {
    type Target = ops::Range<T>;

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

        let (start_str, end_str) = match (range_str.first(), range_str.last()) {
            (Some(f), Some(l)) => match (f.parse::<T>(), l.parse::<T>()) {
                (Ok(s), Ok(e)) => (s, e),
                (Ok(s), Err(_)) => (s, s),
                _ => bail!("Unable to parse range"),
            },
            _ => bail!("Unable to parse range"),
        };

        Ok(Self(start_str..end_str))
    }
}

#[derive(Debug, Default)]
pub struct Urls {
    value: Vec<String>,
}

impl Urls {
    pub fn to_vec(&self) -> Vec<String> {
        self.value.clone()
    }

    pub fn to_query(&self) -> String {
        self.value.join("+")
    }
}

impl FromIterator<String> for Urls {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut c = Urls::default();
        c.value.extend(iter);
        c
    }
}

#[derive(Debug, Default, StructOpt)]
#[structopt(name = "anime-dl", about = "Efficient cli app for downloading anime")]
pub struct Args {
    /// Source urls or scraper's queries
    #[structopt(required_unless("clean"))]
    pub entries: Vec<String>,

    /// Root paths where store files
    #[structopt(default_value = ".", short, long)]
    pub dir: Vec<PathBuf>,

    /// Range of episodes to download
    #[structopt(short, long)]
    pub range: Option<Range<u32>>,

    /// Search anime in remote archive
    #[structopt(
        long,
        short = "S",
        case_insensitive = true,
        possible_values = &Site::variants(),
    )]
    pub search: Option<Option<Site>>,

    /// Find automatically output folder name
    #[structopt(short, long = "auto")]
    pub auto_dir: bool,

    /// Find automatically last episode (override `-r <range>` option)
    #[structopt(short = "c", long = "continue")]
    pub auto_episode: bool,

    /// Override existent files
    #[structopt(short, long)]
    pub force: bool,

    /// Download file without in-app control (equivalent to `curl -O <url>` or `wget <url>`)
    #[structopt(short = "O", long = "one-file")]
    pub single: bool,

    /// Stream episode in a media player (add -O for single file)
    #[structopt(short, long)]
    pub stream: bool,

    /// Interactive mode
    #[structopt(short, long)]
    pub interactive: bool,

    /// Disable automatic proxy (useful for slow conections)
    #[structopt(short = "p", long)]
    pub no_proxy: bool,

    /// Delete app cache
    #[structopt(long)]
    pub clean: bool,

    #[structopt(skip)]
    pub urls: Urls,
}

impl Args {
    pub fn new() -> Self {
        let args = Self::from_args();
        Self {
            urls: Urls::from_iter(args.entries.clone()),
            ..args
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let range1 = Range::new(0, 1);
        let (start, end) = (range1.start, range1.end);
        assert_eq!(start, 0);
        assert_eq!(end, 1);

        let range2 = Range::<i32>::from_str("(0..1)").unwrap();
        assert_eq!(range2.start, 0);
        assert_eq!(range2.end, 1);

        assert!(range1.range().eq(range2.range()));

        let range3 = Range::default();
        assert_eq!((range3.start, range3.end), (1, 0));

        let range4 = Range::<i32>::from_str("1-5").unwrap();
        assert_eq!((range4.start, range4.end), (1, 5));

        let range5 = Range::<i32>::from_str("1-").unwrap();
        assert_eq!((range5.start, range5.end), (1, 1));

        let range6 = Range::<i32>::parse_and_fill("1-", 6).unwrap();
        assert_eq!((range6.start, range6.end), (1, 6));
    }
}
