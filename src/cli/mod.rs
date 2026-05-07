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
    use anyhow::{Result, anyhow};

    use super::Site;
    use crate::{
        anilist::Anilist,
        anime::Anime,
        archives::{AnimeWorld, Archive},
        error::RequestError,
        proxy::ProxyManager,
        scraper::{Scraper, ScraperConfig, Search},
        ui::Tui,
    };

    async fn get_from_watching_list(client: &Anilist) -> Result<Vec<Search>> {
        let Some(list) = client.get_watching_list().await else {
            return Err(anyhow!(RequestError::WatchingList));
        };

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

    pub async fn get_search_results(
        entries: Vec<String>,
        watching: bool,
        anilist_id: Option<u32>,
        no_proxy: bool,
        site: Option<Site>,
    ) -> Result<(Vec<Anime>, &'static str)> {
        let anilist = Anilist::new(anilist_id)?;

        let searches = if watching || entries.is_empty() {
            get_from_watching_list(&anilist).await?
        } else {
            get_from_input(entries)?
        };

        let proxy = ProxyManager::proxy(no_proxy).await;
        let search_result = match site {
            Some(Site::AW) | None => search_site::<AnimeWorld>(&searches, proxy).await?,
        };

        Ok(search_result)
    }
}
