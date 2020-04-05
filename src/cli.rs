use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "animeworld-dl",
    about = "Efficient cli app for downloading anime"
)]
pub struct Cli {
    /// Source url
    #[structopt()]
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

    /// Max number of thread
    #[structopt(default_value = "32", short = "M", long)]
    pub max_threads: usize,

    /// Find automatically last episode (this overrides `-e` option)
    #[structopt(short = "c", long = "continue")]
    pub auto: bool,

    /// Mark anime as finished [WIP]
    #[structopt(short, long)]
    pub finished: bool,
}

impl Cli {
    pub fn new() -> Self {
        Self::from_args()
    }
}
