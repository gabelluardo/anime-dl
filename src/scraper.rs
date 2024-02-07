use std::sync::Arc;

use anyhow::{Context, Result};
use futures::future::join_all;
use owo_colors::OwoColorize;
use rand::seq::IteratorRandom;
use reqwest::{header, header::HeaderValue, Client};
use tokio::sync::Mutex;

use crate::anime::Anime;
use crate::archive::{AnimeWorld, Archive};
use crate::cli::Site;
use crate::errors::{Quit, RemoteError};

#[derive(Debug, Clone)]
pub struct Search {
    pub id: Option<u32>,
    pub string: String,
}

#[derive(Debug)]
pub struct Scraper {
    client: Arc<Client>,
}

impl Scraper {
    pub fn new(proxy: Option<String>) -> Self {
        let user_agent = "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";
        let accept =
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8";

        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, HeaderValue::from_static(accept));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));
        headers.insert(header::USER_AGENT, HeaderValue::from_static(user_agent));
        let mut builder = Client::builder().default_headers(headers);
        if let Some(proxy) = proxy {
            if let Ok(req_proxy) = reqwest::Proxy::http(proxy).context(RemoteError::Proxy) {
                builder = builder.proxy(req_proxy)
            }
        }
        let client = builder.build().unwrap_or_default();

        Self {
            client: Arc::new(client),
        }
    }

    pub async fn run<I>(self, search: I, site: Site) -> Result<(Vec<Anime>, Option<&'static str>)>
    where
        I: Iterator<Item = Search>,
    {
        let (scraper_fun, referrer) = match site {
            Site::AW => (AnimeWorld::run, AnimeWorld::REFERRER),
        };

        let vec = Arc::new(Mutex::new(Vec::new()));
        let tasks = search
            .map(|s| scraper_fun(s.clone(), self.client.clone(), vec.clone()))
            .map(|f| async move {
                if let Err(err) = f.await {
                    if !err.is::<Quit>() {
                        eprintln!("{}", err.red());
                    }
                }
            });
        join_all(tasks).await;

        let anime_vec = vec.lock_owned().await.iter().map(Anime::new).collect();

        Ok((anime_vec, referrer))
    }
}

pub async fn select_proxy(disable: bool) -> Option<String> {
    if disable {
        return None;
    }

    let url = "https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=2000&country=all&ssl=all&anonymity=elite";
    let list = reqwest::get(url).await.ok()?.text().await.ok()?;
    let proxy = list
        .split_ascii_whitespace()
        .choose(&mut rand::thread_rng())
        .map(|s| format!("https://{s}"))
        .unwrap_or_default();

    Some(proxy)
}

#[cfg(test)]
mod tests {
    use super::*;

    use reqwest::Url;

    pub fn get_url(raw_url: &str) -> String {
        Url::parse(raw_url)
            .unwrap()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .into()
    }

    #[tokio::test]
    #[ignore]
    async fn test_remote_scraper() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";

        let site = Site::AW;
        let proxy = select_proxy(false).await;
        let search = vec![Search {
            string: "bunny girl".into(),
            id: None,
        }];

        let (anime, _) = Scraper::new(proxy)
            .run(search.into_iter(), site)
            .await
            .unwrap();
        let info = get_url(&anime.first().unwrap().info.origin);

        assert_eq!(file, info)
    }

    #[tokio::test]
    #[ignore]
    async fn test_remote_scraper_multi() {
        let mut files = vec![
            "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4",
            "TsurezureChildren_Ep_01_SUB_ITA.mp4",
            "Promare_Movie_ITA.mp4",
        ];

        let site = Site::AW;
        let proxy = select_proxy(false).await;
        let search = vec![
            Search {
                string: "bunny girl".into(),
                id: None,
            },
            Search {
                string: "tsuredure children".into(),
                id: None,
            },
            Search {
                string: "promare".into(),
                id: None,
            },
        ];

        let (anime, _) = Scraper::new(proxy)
            .run(search.into_iter(), site)
            .await
            .unwrap();

        let mut anime = anime
            .iter()
            .map(|a| {
                Url::parse(&a.info.origin)
                    .unwrap()
                    .path_segments()
                    .and_then(|segments| segments.last())
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        anime.sort();
        files.sort_unstable();

        assert_eq!(anime, files)
    }
}
