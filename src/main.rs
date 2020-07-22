#[macro_use]
mod macros;

mod anime;
mod cli;
mod utils;

use crate::anime::Manager;
use crate::cli::Args;

#[tokio::main]
async fn main() {
    match Manager::new(Args::new()).run().await {
        Ok(()) => (),
        Err(e) => eprintln!("{}", utils::format_err(e)),
    }
}
