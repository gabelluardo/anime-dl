use std::path::PathBuf;

use crate::range::Range;

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum Site {
    #[default]
    AW,
}

/// Efficient cli app for downloading anime
#[derive(clap::Parser, Debug, Default)]
#[clap(version)]
pub struct Args {
    /// Source urls or scraper's queries
    #[clap(required_unless_present("clean"), required_unless_present("watching"))]
    pub entries: Vec<String>,

    /// Root path where store files
    #[clap(default_value = ".", short, long)]
    pub dir: PathBuf,

    /// Maximum number of simultaneous downloads allowed
    #[clap(
        default_value = "24",
        short = 'm',
        long = "max-concurrent",
        name = "MAX"
    )]
    pub dim_buff: usize,

    /// Episodes to download (es. `1-4` or `1,2,3,4`) [default: 1]
    #[clap(short = 'r', long = "range")]
    pub range: Option<Range<u32>>,

    /// Search anime in remote archive
    #[clap(long, short = 'S', ignore_case = true, value_enum)]
    pub site: Option<Site>,

    /// Save files in a folder with a default name
    #[clap(short = 'D', long = "default-dir")]
    pub auto_dir: bool,

    /// Override existent files
    #[clap(short, long)]
    pub force: bool,

    /// Override app id environment variable    
    #[cfg(feature = "anilist")]
    #[clap(short, long, env = "ANIMEDL_ID", hide_env_values = true)]
    pub anilist_id: Option<u32>,

    /// Retrive anime from anilist watching list        
    #[cfg(feature = "anilist")]
    #[clap(short, long)]
    pub watching: bool,

    /// Stream episode in a media player
    #[clap(short, long, conflicts_with = "range")]
    pub stream: bool,

    /// Interactive mode
    #[clap(short, long, conflicts_with = "range")]
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
