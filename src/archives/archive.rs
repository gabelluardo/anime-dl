use anyhow::Result;
use reqwest::Client;

use crate::{anime::Anime, scraper::Search};

pub trait Archive {
    const REFERRER: &'static str;
    const COOKIE_NAME: &'static str;

    async fn extract_cookie() -> Option<String>;
    async fn search(search: Search, client: Client) -> Result<Vec<Anime>>;
}
