#[macro_use]
mod macros;

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

use owo_colors::OwoColorize;

#[tokio::main]
async fn main() {
    if let Err(err) = cli::run().await {
        eprintln!("{}", err.red());
    }
}
