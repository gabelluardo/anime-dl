use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "animeworld-dl",
    about = "Efficient cli app for downloading anime"
)]
pub struct Cli {
    /// source url
    #[structopt()]
    pub urls: Vec<String>,

    /// where start the downloads
    #[structopt(default_value = "1", short, long)]
    pub start: u32,

    /// path folder where save files
    #[structopt(default_value = ".", short, long)]
    pub dir: Vec<PathBuf>,

    /// progress unless episode exist [WIP]
    #[structopt(short = "c", long = "continue")]
    pub auto: bool,

    /// mark anime as finished [WIP]
    #[structopt(short, long)]
    pub finished: bool,
}

impl Cli {
    pub fn new() -> Self {
        Self::from_args()
    }
}
