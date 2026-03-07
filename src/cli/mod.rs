pub use clap::Parser;

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

    /// Delete app config
    Clean,
}

mod utils {
    use anyhow::{Context, Result};

    use crate::{
        anilist::Anilist,
        anime::Anime,
        archives::Archive,
        scraper::{Scraper, ScraperConfig, Search},
        ui::Tui,
    };

    pub async fn get_from_watching_list(anilist_id: Option<u32>) -> Result<Vec<Search>> {
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

    pub fn get_from_input(entries: Vec<String>) -> Result<Vec<Search>> {
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

    pub async fn search_site<T: Archive>(
        searches: &[Search],
        proxy: Option<String>,
    ) -> Result<(Vec<Anime>, &'static str)> {
        let cookie = T::extract_cookie().await;
        let config = ScraperConfig { proxy, cookie };

        let anime = Scraper::new(config).search::<T>(searches).await?;

        Ok((anime, T::REFERRER))
    }
}
