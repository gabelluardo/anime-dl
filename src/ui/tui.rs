use anyhow::Result;
use indicatif::ProgressBar;
use owo_colors::OwoColorize;

use super::{input, progress::ProgressManager, selector};
use crate::{anilist::WatchingAnime, anime::Anime};

/// Main TUI struct for managing terminal user interface
#[derive(Default)]
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
        println!("{oauth}\n\n{action} {url}\n\n{input}");

        match input::get_command() {
            Ok(input::Command::Default(line)) => line,
            _ => exit(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case(0; "new bar position zero")]
    #[test]
    fn test_tui_new(_dummy: u32) {
        let tui = Tui::new();
        let pb = tui.add_bar();
        assert_eq!(pb.position(), 0);
    }

    #[test_case(0; "default bar position zero")]
    #[test]
    fn test_tui_default(_dummy: u32) {
        let tui = Tui::default();
        let pb = tui.add_bar();
        assert_eq!(pb.position(), 0);
    }

    #[test_case(2; "two bars")]
    #[test_case(3; "three bars")]
    #[test]
    fn test_tui_add_bar_multiple(count: usize) {
        let tui = Tui::new();
        for _ in 0..count {
            let pb = tui.add_bar();
            assert_eq!(pb.position(), 0);
        }
    }
}
