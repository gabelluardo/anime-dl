use structopt::clap::arg_enum;
use structopt::StructOpt;

use std::iter::FromIterator;
use std::path::PathBuf;
use std::str::FromStr;

arg_enum! {
    #[derive(Debug, Copy, Clone)]
    pub enum Site {
        AW,
        AS,
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Range {
    pub start: u32,
    pub end: u32,
}

impl Range {
    pub fn from((start, end): (u32, u32)) -> Self {
        Self { start, end }
    }

    pub fn extract(&self) -> (u32, u32) {
        (self.start, self.end)
    }
}

impl FromStr for Range {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .collect::<Vec<_>>();

        let start_fromstr = coords[0].parse::<u32>()?;
        let end_fromstr = coords[1].parse::<u32>()?;

        Ok(Range {
            start: start_fromstr,
            end: end_fromstr,
        })
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
    #[structopt(required = true)]
    pub entries: Vec<String>,

    /// Root paths where store files
    #[structopt(default_value = ".", short, long)]
    pub dir: Vec<PathBuf>,

    /// Range of episodes to download
    #[structopt(short, long)]
    pub range: Option<Range>,

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

    /// Search anime in remote archive
    #[structopt(
        long,
        short = "S",
        possible_values = &Site::variants()
    )]
    pub search: Option<Site>,

    /// Stream episode in a media player (add -O for single file)
    #[structopt(short, long)]
    pub stream: bool,

    /// Interactive mode
    #[structopt(short, long)]
    pub interactive: bool,

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
        let range1 = Range { start: 0, end: 1 };
        let (start, end) = range1.extract();

        assert_eq!(start, 0);
        assert_eq!(end, 1);

        let range2 = Range::from_str("(0,1)").unwrap();
        let (start, end) = range2.extract();

        assert_eq!(start, 0);
        assert_eq!(end, 1);

        assert_eq!(range1.extract(), range2.extract());
    }
}
