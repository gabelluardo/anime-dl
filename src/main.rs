mod anilist;
mod anime;
mod app;
mod archives;
mod cli;
mod config;
mod error;
mod proxy;
mod range;
mod scraper;
mod ui;

use cli::{Args, Parser};
use owo_colors::OwoColorize;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(err) = app::run(args).await {
        eprintln!("{}", err.red());
    }
}
