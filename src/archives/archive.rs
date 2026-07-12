use std::future::Future;

use anyhow::Result;
use reqwest::Client;

use crate::{anilist::AnilistId, anime::Anime, scraper::Search};

pub trait Archive {
    const REFERRER: &'static str;

    fn get_session_id() -> impl Future<Output = Result<String>> + Send;
    fn search(
        search: Search,
        client: Client,
        anilist_id: Option<AnilistId>,
    ) -> impl Future<Output = Result<Vec<Anime>>> + Send;
}
