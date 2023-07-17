#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;

#[cfg(feature = "anilist")]
mod anilist;

mod anime;
mod app;
mod archive;
mod cli;
mod errors;
mod file;
mod range;
mod scraper;
mod tui;
mod utils;

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
