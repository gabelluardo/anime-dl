use anime_dl::cli::{Args, Parser};
use owo_colors::OwoColorize;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(err) = anime_dl::app::run(args).await {
        eprintln!("Error: {}", err.red());
    }
}
