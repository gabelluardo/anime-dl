use anyhow::Result;
use clap::Parser;

use crate::{
    anilist::{self, WatchingAnime},
    anime::{Anime, AnimeInfo},
    config::clean_config,
    errors::RemoteError,
    parser,
    scraper::{Scraper, Search},
    tui,
};

mod download;
mod stream;

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum Site {
    #[default]
    AW,
}

/// Efficient cli app for downloading anime
#[derive(Parser, Debug, Default)]
#[command(author, version, about, arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Parser, Debug)]
enum Command {
    #[command(alias = "s")]
    Stream(stream::Args),
    #[command(alias = "d")]
    Download(download::Args),

    #[cfg(feature = "anilist")]
    /// Delete app cache
    Clean,
}

pub async fn run() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Command::Stream(cmd)) => stream::execute(cmd).await,
        Some(Command::Download(cmd)) => download::execute(cmd).await,

        #[cfg(feature = "anilist")]
        Some(Command::Clean) => clean_config(),
        _ => unreachable!(),
    }
}

async fn get_from_watching_list(
    anilist_id: Option<u32>,
    proxy: Option<String>,
    site: Site,
) -> Result<(Vec<Anime>, Option<&'static str>)> {
    let mut series = anilist::get_watching_list(anilist_id)
        .await
        .ok_or(RemoteError::WatchingList)?;
    tui::watching_choice(&mut series)?;

    let search = series.iter().map(|WatchingAnime { title, id, .. }| {
        let string = title
            .split_ascii_whitespace()
            .take(2)
            .fold(String::new(), |acc, s| acc + "+" + s.trim());

        Search {
            string,
            id: Some(*id),
        }
    });

    Scraper::new(proxy).run(search, site).await
}

async fn get_from_input(
    entries: Vec<String>,
    proxy: Option<String>,
    site: Site,
) -> Result<(Vec<Anime>, Option<&'static str>)> {
    if let Some(entry) = entries.first() {
        if parser::is_web_url(entry) {
            let items = entries
                .iter()
                .map(|s| {
                    let name = to_title_case!(parser::parse_name(s).unwrap_or_default());
                    let info = AnimeInfo::new(&name, s, None, None);

                    Anime::new(&info)
                })
                .collect();

            return Ok((items, None));
        }
    }

    let input = &entries.join(" ");
    let search = input
        .split(',')
        .map(|s| s.trim().replace(' ', "+"))
        .map(|s| Search {
            string: s,
            id: None,
        });

    Scraper::new(proxy).run(search, site).await
}
