use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;

use crate::utils::Range;

#[derive(clap::ValueEnum, Debug, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum Site {
    AW,
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

#[derive(clap::Parser, Debug, Default)]
#[clap(
    name = "anime-dl",
    about = "Efficient cli app for downloading anime",
    version
)]
pub struct Args {
    /// Source urls or scraper's queries
    #[clap(required_unless_present("clean"))]
    pub entries: Vec<String>,

    /// Root paths where store files
    #[clap(default_value = ".", short, long)]
    pub dir: Vec<PathBuf>,

    /// Maximum number of simultaneous downloads allowed
    #[clap(
        default_value = "24",
        short = 'm',
        long = "max-concurrent",
        name = "max",
        value_parser = parse_dim_buff
    )]
    pub dim_buff: usize,

    /// Episodes to download (es. `1-4` or `1,2,3,4`) [default: 1]
    #[clap(
        short = 'r',
        long = "range",
        name = "range",
        required_unless_present("auto_episode"),
        required_unless_present("interactive"),
        required_unless_present("stream")
    )]
    pub range: Option<Range<u32>>,

    /// Search anime in remote archive
    #[clap(long, short = 'S', name = "site", ignore_case = true, value_enum)]
    pub site: Option<Site>,

    /// Save files in a folder with a default name
    #[clap(short = 'D', long = "default-dir")]
    pub auto_dir: bool,

    /// Find automatically last episode
    #[clap(short = 'c', long = "continue")]
    pub auto_episode: bool,

    /// Override existent files
    #[clap(short, long)]
    pub force: bool,

    /// Override app id environment variable
    #[clap(short, long, env = "ANIMEDL_ID", hide_env_values = true)]
    pub anilist_id: Option<u32>,

    /// Stream episode in a media player
    #[clap(short, long)]
    pub stream: bool,

    /// Interactive mode
    #[clap(short, long)]
    pub interactive: bool,

    /// Disable automatic proxy (useful for slow connections)
    #[clap(short = 'p', long)]
    pub no_proxy: bool,

    /// Delete app cache
    #[clap(long)]
    pub clean: bool,
}

impl Args {
    pub fn parse() -> Self {
        clap::Parser::parse()
    }
}
