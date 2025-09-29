use std::collections::VecDeque;

use anyhow::{Context, Result};
pub use clap::Parser;

use crate::{
    anilist::{Anilist, WatchingAnime},
    scraper::Search,
    tui::Tui,
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
struct EpisodeProgress {
    anime_id: Option<u32>,
    episode: Option<u32>,
    percentage: Option<u32>,
    updated: bool,
    count: u32,
}

#[derive(Default, Debug)]
struct Progress {
    anilist: Anilist,
    queue: VecDeque<EpisodeProgress>,
}

impl Progress {
    pub fn new(anilist: Anilist) -> Self {
        Self {
            anilist,
            ..Default::default()
        }
    }

    pub fn push(&mut self, anime_id: Option<u32>, episode: Option<u32>) {
        self.queue.push_back(EpisodeProgress {
            anime_id,
            episode,
            ..Default::default()
        });
    }

    pub fn percentage(&mut self, percentage: Option<u32>) {
        match (self.queue.front_mut(), percentage) {
            // update current episode
            (Some(p), Some(_)) if p.percentage <= percentage => {
                p.percentage = percentage;
                p.count += 1;
            }

            // new episode is selected, pass to the next
            (Some(EpisodeProgress { updated: true, .. }), Some(0)) => {
                self.queue.pop_front();
            }

            _ => (),
        }
    }

    pub async fn update(&mut self) {
        if self.to_update() {
            if let Some(progress) = self.queue.front_mut() {
                if let Some(number) = progress.episode {
                    let result = self.anilist.update(progress.anime_id, number).await;
                    progress.updated = result.is_ok();
                }
            }
        }
    }

    fn to_update(&self) -> bool {
        if let Some(p) = self.queue.front() {
            return !p.updated && p.count >= 5 && p.percentage.is_some_and(|p| p > 80);
        }

        false
    }
}

async fn get_from_watching_list(anilist_id: Option<u32>) -> Result<Vec<Search>> {
    let list = Anilist::new(anilist_id)?
        .get_watching_list()
        .await
        .context("Unable to get data from watching list")?;

    let search = Tui::select_from_watching(&list)?
        .iter()
        .map(|WatchingAnime { title, id, .. }| Search {
            string: title
                .split_ascii_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join("+"),
            id: Some(*id),
        })
        .collect();

    Ok(search)
}

fn get_from_input(entries: Vec<String>) -> Result<Vec<Search>> {
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
