use crate::utils::Range;

use structopt::{clap::arg_enum, StructOpt};

use std::iter::FromIterator;
use std::ops::Deref;
use std::path::PathBuf;

arg_enum! {
    #[derive(Debug, Copy, Clone)]
    pub enum Site {
        AW,
        AS,
    }
}

#[derive(Debug, Default)]
pub struct Urls(Vec<String>);

impl Deref for Urls {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Urls {
    pub fn to_query(&self) -> String {
        self.0.join("+")
    }
}

impl FromIterator<String> for Urls {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut c = Urls::default();
        c.0.extend(iter);
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

    /// Maximum number of simultaneous downloads allowed
    #[structopt(
        default_value = "24",
        short = "m",
        long = "max-concurrent",
        name = "max"
    )]
    pub dim_buff: usize,

    /// Range of episodes to download
    #[structopt(
        short = "r",
        long = "range",
        name = "range",
        required_unless("single"),
        required_unless("stream"),
        required_unless("interactive"),
        required_unless("auto-episode")
    )]
    pub opt_range: Option<Range<u32>>,

    /// Search anime in remote archive
    #[structopt(
        long,
        short = "S",
        name = "site",
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

    /// Disable automatic proxy (useful for slow connections)
    #[structopt(short = "p", long)]
    pub no_proxy: bool,

    /// Delete app cache
    #[structopt(long)]
    pub clean: bool,

    #[structopt(skip)]
    pub urls: Urls,

    #[structopt(skip)]
    pub range: Range<u32>,
}

impl Args {
    pub fn parse() -> Self {
        let args = Self::from_args();

        let urls = Urls::from_iter(args.entries.clone());
        let dim_buff = if args.dim_buff == 0 { 1 } else { args.dim_buff };
        let range = match &args.opt_range {
            Some(range) => range.to_owned(),
            None => Range::default(),
        };

        Self {
            urls,
            range,
            dim_buff,
            ..args
        }
    }
}
