#[macro_use]
mod macros;

mod anime;
mod cli;
mod scraper;
mod utils;

use anime::Manager;
use cli::Args;

#[tokio::main]
async fn main() {
    match Manager::from(Args::new()).run().await {
        Ok(()) => (),
        Err(e) => eprintln!("{}", utils::format_err(e)),
    }
}
