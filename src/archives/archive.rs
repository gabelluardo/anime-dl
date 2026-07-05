use std::future::Future;

use anyhow::Result;
use reqwest::Client;

use crate::{anilist::AnilistId, anime::Anime, scraper::Search};

pub trait Archive {
    const REFERRER: &'static str;
    const COOKIE_NAME: &'static str;

    fn extract_cookie() -> impl Future<Output = Option<String>> + Send;
    fn search(
        search: Search,
        client: Client,
        anilist_id: Option<AnilistId>,
    ) -> impl Future<Output = Result<Vec<Anime>>> + Send;
}
