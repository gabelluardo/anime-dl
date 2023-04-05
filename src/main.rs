#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod utils;

#[cfg(feature = "anilist")]
mod anilist;

mod anime;
mod app;
mod cli;
mod errors;
mod scraper;

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
