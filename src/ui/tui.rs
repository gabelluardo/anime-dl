use anyhow::Result;
use indicatif::ProgressBar;
use owo_colors::OwoColorize;

use super::{input, progress::ProgressManager, selector};
use crate::{anilist::WatchingAnime, anime::Anime};

/// Main TUI struct for managing terminal user interface
pub struct Tui {
    progress: ProgressManager,
}

impl Tui {
    pub fn new() -> Self {
        Self {
            progress: ProgressManager::new(),
        }
    }

    pub fn add_bar(&self) -> ProgressBar {
        self.progress.add_bar()
    }

    pub fn select_from_watching(series: &[WatchingAnime]) -> Result<Vec<&WatchingAnime>> {
        selector::select_from_watching(series)
    }

    pub fn select_series(series: &mut Vec<Anime>) -> Result<()> {
        selector::select_series(series)
    }

    pub fn select_episodes(anime: &Anime) -> Result<Vec<String>> {
        selector::select_episodes(anime)
    }

    pub fn get_token(url: &str) -> String {
        use std::process::exit;

        let oauth = "Anilist Oauth".cyan().bold().to_string();
        let action = "Authenticate to:".green().to_string();
        let url = url.magenta().bold().to_string();
        let input = ":: ".red().to_string() + &"Paste token here:".bold().to_string();
        let text = oauth + "\n\n" + &action + " " + &url + "\n\n" + &input;
        println!("{text}");

        match input::get_command() {
            Ok(input::Command::Default(line)) => line,
            _ => exit(0),
        }
    }
}
