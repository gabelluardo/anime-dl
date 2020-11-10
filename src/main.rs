#[macro_use]
mod macros;

mod anime;
mod api;
mod cli;
mod scraper;
mod utils;

use anime::Manager;
use cli::Args;

#[tokio::main]
async fn main() {
    match Manager::new(Args::new()).run().await {
        Ok(_) => (),
        Err(e) => bunt::eprintln!("{$red}[ERR] {}{/$}", e),
    }
}
