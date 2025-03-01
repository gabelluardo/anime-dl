use anyhow::{Context, Result};
pub use clap::Parser;

use crate::{
    anilist::{self, WatchingAnime},
    scraper::Search,
    tui,
};

pub mod download;
pub mod stream;

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum Site {
    #[default]
    AW,
}

/// Efficient cli app for downloading anime
#[derive(Parser, Debug)]
#[command(author, version, about, arg_required_else_help = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    #[command(alias = "s")]
    Stream(stream::Args),
    #[command(alias = "d")]
    Download(download::Args),

    #[cfg(feature = "anilist")]
    /// Delete app cache
    Clean,
}

#[derive(Default, Debug)]
struct Progress {
    anime_id: Option<u32>,
    episode: Option<u32>,
    percentage: Option<u32>,
    count: u32,
    updated: bool,
}

impl Progress {
    fn new() -> Self {
        Progress::default()
    }

    fn anime_id(&mut self, id: Option<u32>) -> &mut Self {
        self.anime_id = id;
        self.count = 0;
        self
    }

    fn episode(&mut self, ep: Option<u32>) -> &mut Self {
        self.episode = ep;
        self.updated = false;
        self
    }

    fn percentage(&mut self, perc: Option<u32>) {
        self.percentage = perc;
        self.count += 1;
    }

    fn updated(&mut self, updated: bool) {
        self.updated = updated;
    }

    fn to_update(&self) -> Option<u32> {
        match self.episode {
            ep if self.percentage >= Some(80) && self.count >= 5 => ep,
            _ => None,
        }
    }

    fn is_updated(&self) -> bool {
        self.updated
    }
}

async fn get_from_watching_list(anilist_id: Option<u32>) -> Result<Vec<Search>> {
    let mut series = anilist::get_watching_list(anilist_id)
        .await
        .context("Unable to get data from watching list")?;

    tui::watching_choice(&mut series)?;

    let search = series
        .iter()
        .map(|WatchingAnime { title, id, .. }| {
            let string = title
                .split_ascii_whitespace()
                .take(2)
                .fold(String::new(), |acc, s| acc + "+" + s.trim());

            Search {
                string,
                id: Some(*id),
            }
        })
        .collect();

    Ok(search)
}

async fn get_from_input(entries: Vec<String>) -> Result<Vec<Search>> {
    // if let Some(entry) = entries.first() {
    //     if parser::is_web_url(entry) {
    //         let items = entries
    //             .iter()
    //             .map(|s| {
    //                 let name = to_title_case!(parser::parse_name(s).unwrap_or_default());
    //                 let info = AnimeInfo::new(&name, s, None, None);

    //                 Anime::new(&info)
    //             })
    //             .collect();

    //         return Ok((items, None));
    //     }
    // }

    let input = &entries.join(" ");
    let search = input
        .split(',')
        .map(|s| s.trim().replace(' ', "+"))
        .map(|s| Search {
            string: s,
            id: None,
        })
        .collect();

    Ok(search)
}
