use anyhow::{Context, Result};
pub use clap::Parser;

use crate::{
    anilist::{Anilist, WatchingAnime},
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
    anilist: Anilist,
    anime_id: Option<u32>,
    episode: Option<u32>,
    percentage: Option<u32>,
    count: u32,
    updated: bool,
}

impl Progress {
    fn new(anilist: Anilist) -> Self {
        Self {
            anilist,
            ..Default::default()
        }
    }

    fn anime_id(&mut self, id: Option<u32>) {
        self.anime_id = id;
        self.count = 0;
    }

    fn episode(&mut self, ep: Option<u32>) {
        self.episode = ep;
        self.updated = false;
    }

    fn percentage(&mut self, percentage: Option<u32>) {
        self.percentage = percentage;
        self.count += 1;
    }

    async fn update(&mut self) {
        if self.to_update() {
            if let Some(number) = self.episode {
                let result = self.anilist.update(self.anime_id, number).await;
                self.updated = result.is_ok();
            }
        }
    }

    fn to_update(&self) -> bool {
        !self.updated && self.count >= 5 && self.percentage.is_some_and(|p| p > 80)
    }
}

async fn get_from_watching_list(anilist_id: Option<u32>) -> Result<Vec<Search>> {
    let list = Anilist::new(anilist_id)?
        .get_watching_list()
        .await
        .context("Unable to get data from watching list")?;

    let search = tui::watching_choice(&list)?
        .iter()
        .map(|WatchingAnime { title, id, .. }| Search {
            string: title
                .split_ascii_whitespace()
                .take(3)
                .fold(String::new(), |acc, s| acc + "+" + s),
            id: Some(*id),
        })
        .collect();

    Ok(search)
}

async fn get_from_input(entries: Vec<String>) -> Result<Vec<Search>> {
    let search = entries
        .join(" ")
        .split(',')
        .map(|s| s.trim().replace(' ', "+"))
        .map(|s| Search {
            string: s,
            id: None,
        })
        .collect();

    Ok(search)
}
