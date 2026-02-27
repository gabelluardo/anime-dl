use std::collections::VecDeque;

use anyhow::{Context, Result};
pub use clap::Parser;

use crate::{
    anilist::Anilist,
    anime::Anime,
    archive::Archive,
    scraper::{Scraper, ScraperConfig, Search},
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

    /// Delete app cache
    Clean,
}

#[derive(Default, Debug)]
struct EpisodeProgress {
    anime_id: Option<u32>,
    episode: Option<u32>,
    percentage: Option<u32>,
    updated: bool,
}

#[derive(Default, Debug)]
struct Progress {
    anilist: Anilist,
    queue: VecDeque<EpisodeProgress>,
}

const MIN_PERCENTAGE: u32 = 80;

impl Progress {
    pub fn new(anilist: Anilist) -> Self {
        let queue = VecDeque::new();

        Self { anilist, queue }
    }

    pub fn track(&mut self, anime_id: Option<u32>, episode: Option<u32>) {
        let Self { queue, .. } = self;

        let progess = EpisodeProgress {
            anime_id,
            episode,
            percentage: None,
            updated: false,
        };

        queue.push_back(progess);
    }

    pub fn update(&mut self, percentage: Option<u32>) {
        let Self { queue, .. } = self;

        match (queue.front_mut(), percentage) {
            // update current episode
            (Some(p), Some(_)) if p.percentage <= percentage => {
                p.percentage = percentage;
            }

            // new episode is selected, pass to the next
            (Some(EpisodeProgress { updated: true, .. }), Some(0)) => {
                queue.pop_front();
            }

            _ => (),
        }
    }

    pub async fn send(&mut self) {
        fn need_update(queue: &mut VecDeque<EpisodeProgress>) -> bool {
            if let Some(p) = queue.front() {
                return !p.updated && p.percentage.is_some_and(|p| p > MIN_PERCENTAGE);
            }

            false
        }

        let Self { anilist, queue } = self;

        if need_update(queue)
            && let Some(progress) = queue.front_mut()
            && let Some(number) = progress.episode
            && let Some(id) = progress.anime_id
        {
            let result = anilist.update(id, number).await;
            progress.updated = result.is_ok();
        }
    }
}

async fn get_from_watching_list(anilist_id: Option<u32>) -> Result<Vec<Search>> {
    let list = Anilist::new(anilist_id)?
        .get_watching_list()
        .await
        .context("Unable to get data from watching list")?;

    let search = Tui::select_from_watching(&list)?
        .iter()
        .map(|info| {
            let id = Some(info.id());
            let string = info
                .title()
                .split_ascii_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join("+");

            Search { string, id }
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

async fn search_site<T: Archive>(
    searches: &[Search],
    proxy: Option<String>,
) -> Result<(Vec<Anime>, &'static str)> {
    let cookie = T::extract_cookie().await;
    let config = ScraperConfig { proxy, cookie };

    let anime = Scraper::new(config).search::<T>(searches).await?;

    Ok((anime, T::REFERRER))
}
