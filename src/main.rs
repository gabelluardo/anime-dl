#[macro_use]
mod macros;

#[cfg(feature = "anilist")]
mod anilist;

mod anime;
mod app;
mod archive;
mod cli;
mod config;
mod errors;
mod parser;
mod range;
mod scraper;
mod tui;

use app::App;
use owo_colors::OwoColorize;

#[tokio::main]
async fn main() {
    if let Err(err) = App::run().await {
        if !err.is::<errors::Quit>() {
            eprintln!("{}", err.red());
        }
    }
}
