use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;

use structopt::{clap::arg_enum, StructOpt};

use crate::utils::Range;

arg_enum! {
    #[derive(Debug, Copy, Clone)]
    #[allow(clippy::upper_case_acronyms)]
    pub enum Site {
        AW,
    }
}

impl Default for Site {
    fn default() -> Self {
        Site::AW
    }
}

fn parse_dim_buff(src: &str) -> Result<usize, ParseIntError> {
    let mut num = usize::from_str(src)?;
    if num == 0 {
        num = 1
    }

    Ok(num)
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
        name = "max",
        parse(try_from_str = parse_dim_buff)
    )]
    pub dim_buff: usize,

    /// Episodes to download (es. `1-4` or `1,2,3,4`) [default: 1]
    #[structopt(
        short = "r",
        long = "range",
        name = "range",
        required_unless("auto-episode"),
        required_unless("clean"),
        required_unless("interactive"),
        required_unless("stream")
    )]
    pub range: Option<Range<u32>>,

    /// Search anime in remote archive
    #[structopt(
        long,
        short = "S",
        name = "site",
        case_insensitive = true,
        possible_values = & Site::variants(),
    )]
    pub site: Option<Site>,

    /// Save files in a folder with a default name
    #[structopt(short = "D", long = "default-dir")]
    pub auto_dir: bool,

    /// Find automatically last episode
    #[structopt(short = "c", long = "continue")]
    pub auto_episode: bool,

    /// Override existent files
    #[structopt(short, long)]
    pub force: bool,

    /// Override app id environment variable
    #[structopt(short, long, env = "ANIMEDL_ID", hide_env_values = true)]
    pub anilist_id: Option<u32>,

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
    pub fn from_args() -> Self {
        StructOpt::from_args()
    }
}
