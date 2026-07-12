use anyhow::Result;
use indicatif::ProgressBar;

use super::{progress::ProgressManager, selector};
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

    pub fn get_token(url: &str) -> Result<String> {
        #[cfg(test)]
        {
            let _ = url;
            Ok(String::new())
        }

        #[cfg(not(test))]
        {
            use std::process::exit;

            use super::input;

            use owo_colors::OwoColorize;

            let oauth = "Anilist Oauth".cyan().bold().to_string();
            let action = "Authenticate to:".green().to_string();
            let url = url.magenta().bold().to_string();
            let input = ":: ".red().to_string() + &"Paste token here:".bold().to_string();
            println!("{oauth}\n\n{action} {url}\n\n{input}");

            match input::get_command()? {
                input::Command::Default(line) => Ok(line),
                _ => exit(0),
            }
        }
    }

    pub fn get_session_id(archive: &str) -> Result<String> {
        #[cfg(test)]
        {
            let _ = archive;
            Ok(String::new())
        }

        #[cfg(not(test))]
        {
            use std::process::exit;

            use super::input;

            use anyhow::bail;
            use owo_colors::OwoColorize;

            // If not found, prompt user and save
            let prompt_msg = format!("{} Session ID", archive).cyan().bold().to_string();
            let instruction = "Enter your session ID (check F12 → Application → Cookies):"
                .green()
                .to_string();
            let input_hint = ":: ".red().to_string() + &"Session ID:".bold().to_string();
            println!("{prompt_msg}\n\n{instruction}\n\n{input_hint}");

            match input::get_command()? {
                input::Command::Default(line) if !line.is_empty() => Ok(line),
                input::Command::Quit => exit(0),
                _ => bail!(crate::error::RequestError::SessionId),
            }
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

    #[test]
    fn test_get_token_returns_empty_in_test() {
        let result = Tui::get_token("https://anilist.co/api/v2/oauth/authorize");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test_case("AnimeWorld"; "animeworld archive")]
    #[test_case("SomeOtherArchive"; "other archive")]
    #[test]
    fn test_get_session_id_returns_empty_in_test(archive: &str) {
        let result = Tui::get_session_id(archive);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }
}
