use structopt::clap::arg_enum;
use structopt::StructOpt;

use std::path::PathBuf;


arg_enum! {
    #[derive(Debug)]
    pub enum Site {
        AW,
        AS,
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "anime-dl", about = "Efficient cli app for downloading anime")]
pub struct Cli {
    /// Source url
    #[structopt(required = true)]
    pub urls: Vec<String>,

    /// Path folder where save files
    #[structopt(default_value = ".", short, long)]
    pub dir: Vec<PathBuf>,

    /// First episode to download
    #[structopt(default_value = "1", short, long)]
    pub start: u32,

    /// Last episode to download
    #[structopt(default_value, short, long)]
    pub end: u32,

    /// [WIP] Max number of concurrent downloads
    #[structopt(default_value = "32", short = "M", long)]
    pub max_threads: usize,

    /// Find automatically output folder name
    #[structopt(short = "a", long = "auto")]
    pub auto_dir: bool,

    /// Find automatically last episode (this overrides `-e` option)
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

impl Cli {
    pub fn new() -> Self {
        Self::from_args()
    }
}
