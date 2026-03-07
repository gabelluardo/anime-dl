use anyhow::Result;

use crate::{
    cli::{Args, Command, download, stream},
    config::clean,
};

pub async fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Stream(cmd) => stream::exec(cmd).await,
        Command::Download(cmd) => download::exec(cmd).await,
        Command::Clean => clean(),
    }
}
