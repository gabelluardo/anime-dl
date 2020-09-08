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
    match Manager::from(Args::new()).run().await {
        Ok(_) => (),
        Err(e) => eprintln!("{}", utils::tui::format_err(e)),
    }
}
