#[macro_use]
mod macros;

mod anime;
mod cli;
mod utils;

use crate::anime::Manager;
use crate::cli::Args;

#[tokio::main]
async fn main() {
    print_err!(Manager::new(Args::new()).run().await)
}
