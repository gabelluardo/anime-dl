pub mod animeworld;

use anyhow::Result;
use reqwest::Client;
use scraper::Html;

use crate::anime::Anime;
use crate::scraper::Search;

pub use animeworld::AnimeWorld;

pub trait Archive {
    const REFERRER: &'static str;
    const COOKIE_NAME: &'static str;

    async fn extract_cookie() -> Option<String>;
    async fn search(search: Search, client: Client) -> Result<Vec<Anime>>;
}

mod selector {
    use super::*;

    use scraper::Selector;

    pub fn from(s: &str) -> Selector {
        match Selector::parse(s) {
            Ok(s) => s,
            Err(_) => panic!("unable to parse selector {s}"),
        }
    }

    pub async fn get_page(client: &Client, url: &str) -> Result<Html> {
        let response = client.get(url).send().await?.error_for_status()?;
        let fragment = Html::parse_fragment(&response.text().await?);

        Ok(fragment)
    }
}
