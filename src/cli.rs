use structopt::clap::arg_enum;
use structopt::StructOpt;

use std::path::PathBuf;

arg_enum! {
    #[derive(Debug, Copy, Clone)]
    pub enum Site {
        AW,
        AS,
    }
}

#[derive(Debug)]
pub struct Range {
    start: u32,
    end: u32
}

impl Range {
    pub fn extract(&self) -> (u32, u32){
        (self.start, self.end)
    }
}

impl std::str::FromStr for Range {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords = s.trim_matches(|p| p == '(' || p == ')' )
                                 .split(',')
                                 .collect::<Vec<_>>();

        let start_fromstr = coords[0].parse::<u32>()?;
        let end_fromstr = coords[1].parse::<u32>()?;

        Ok(Range { start: start_fromstr, end: end_fromstr })
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "anime-dl", about = "Efficient cli app for downloading anime")]
pub struct Args {
    /// Source url
    #[structopt(required = true)]
    pub urls: Vec<String>,

    /// Root folders where save files
    #[structopt(default_value = ".", short, long)]
    pub dir: Vec<PathBuf>,

    /// Range of episodes to download
    #[structopt(short, long)]
    pub range: Option<Range>,

    // /// [WIP] Max number of concurrent downloads
    // #[structopt(default_value = "32", short = "M", long)]
    // pub max_threads: usize,

    /// Find automatically output folder name
    #[structopt(short = "a", long = "auto")]
    pub auto_dir: bool,

    /// Find automatically last episode (override `-r <range>` option)
    #[structopt(short = "c", long = "continue")]
    pub auto_episode: bool,

    /// Override existent files
    #[structopt(short, long)]
    pub force: bool,

    /// Download only the file form the url (equivalent to `curl -O <url>`)
    #[structopt(short = "O", long = "one-file")]
    pub single: bool,

    /// Search anime in remote archive
    #[structopt(
        long,
        short = "S",
        possible_values = &Site::variants(), 
    )]
    pub search: Option<Site>,
}

impl Args {
    pub fn new() -> Self {
        Self::from_args()
    }
}
