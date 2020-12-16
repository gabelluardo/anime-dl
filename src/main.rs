#[macro_use]
mod macros;

mod anime;
mod api;
mod cli;
mod scraper;
mod utils;

use anime::Manager;
use anyhow::Result;
use cli::Args;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => (),
        Err(e) => bunt::eprintln!("{$red}[ERR] {}{/$}", e),
    }
}

async fn run() -> Result<()> {
    Manager::new(Args::parse()).await?.run().await
}
