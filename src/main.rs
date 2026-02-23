#[cfg(feature = "anilist")]
mod anilist;

mod anime;
mod archive;
mod cli;
mod config;
mod parser;
mod range;
mod scraper;
mod tui;

use cli::{Args, Command, Parser, download, stream};
use config::clean_config;
use owo_colors::OwoColorize;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let result = match args.command {
        Command::Stream(cmd) => stream::exec(cmd).await,
        Command::Download(cmd) => download::exec(cmd).await,
        #[cfg(feature = "anilist")]
        Command::Clean => clean_config(),
    };

    if let Err(err) = result {
        eprintln!("{}", err.red());
    }
}
