mod anilist;
mod anime;
mod archives;
mod cli;
mod config;
mod error;
mod proxy;
mod range;
mod scraper;
mod ui;

use anyhow::Result;
use cli::{Args, Command, Parser, download, stream};
use config::clean;
use owo_colors::OwoColorize;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(err) = run(args).await {
        eprintln!("{}", err.red());
    }
}

async fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Stream(cmd) => stream::exec(cmd).await,
        Command::Download(cmd) => download::exec(cmd).await,
        Command::Clean => clean(),
    }
}
