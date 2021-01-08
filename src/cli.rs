use crate::utils::Range;

use structopt::{clap::arg_enum, StructOpt};

use std::path::PathBuf;

arg_enum! {
    #[derive(Debug, Copy, Clone)]
    pub enum Site {
        AW,
        AS,
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

    /// Episodes to download (es. `1-4` or `1,2,3,4`) [default: 1]
    #[structopt(
        short = "r",
        long = "range",
        name = "range",
        required_unless("single"),
        required_unless("stream"),
        required_unless("interactive"),
        required_unless("auto-episode")
    )]
    pub range: Option<Range<u32>>,

    /// Search anime in remote archive
    #[structopt(
        long,
        short = "S",
        name = "site",
        case_insensitive = true,
        possible_values = &Site::variants(),
    )]
    pub search: Option<Option<Site>>,

    /// Save files in a folder with a default name
    #[structopt(short = "D", long = "default-dir")]
    pub auto_dir: bool,

    /// Find automatically last episode
    #[structopt(short = "c", long = "continue")]
    pub auto_episode: bool,

    /// Override existent files
    #[structopt(short, long)]
    pub force: bool,

    /// Download file without in-app control [Deprecated since 1.2.0]
    #[structopt(short = "O", long = "one-file")]
    pub single: bool,

    /// Stream episode in a media player
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
}

impl Args {
    pub fn parse() -> Self {
        let mut args = Self::from_args();

        args.dim_buff = match args.dim_buff {
            0 => 1,
            _ => args.dim_buff,
        };

        args
    }
}
