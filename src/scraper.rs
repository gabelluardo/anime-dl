use std::sync::Arc;

use anyhow::Result;
use futures::future::join_all;
use owo_colors::OwoColorize;
use rand::seq::IteratorRandom;
use reqwest::{Client, header, header::HeaderValue};
use tokio::sync::Mutex;

use crate::anime::Anime;
use crate::archive::{AnimeWorld, Archive};
use crate::cli::Site;

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
    pub fn new(proxy: Option<String>, cookie: Option<String>) -> Self {
        let mut headers = header::HeaderMap::new();

        if let Some(cookie) = cookie {
            if let Ok(value) = HeaderValue::from_str(&cookie) {
                headers.insert(header::COOKIE, value);
            }
        }

        let mut builder = Client::builder()
            .default_headers(headers)
            .danger_accept_invalid_certs(true);
        if let Some(proxy) = proxy {
            if let Ok(req_proxy) = reqwest::Proxy::http(proxy) {
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
        let (scrape, referrer) = match site {
            Site::AW => (AnimeWorld::run, AnimeWorld::REFERRER),
        };

        let vec = Arc::new(Mutex::new(Vec::new()));
        let tasks = search.map(async |s| {
            if let Err(err) = scrape(s, self.client.clone(), vec.clone()).await {
                eprintln!("{}", err.red());
            }
        });
        join_all(tasks).await;

        let anime = vec.lock_owned().await.to_vec();

        Ok((anime, referrer))
    }

    #[cfg(test)]
    pub fn client(&self) -> Arc<Client> {
        self.client.clone()
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
        .choose(&mut rand::rng())
        .map(|s| format!("https://{s}"))
        .unwrap_or_default();

    Some(proxy)
}

pub async fn find_cookie(site: Site) -> Option<String> {
    let (str, url) = match site {
        Site::AW => ("SecurityAW", AnimeWorld::REFERRER?),
    };

    let security = reqwest::get(url).await.ok()?.text().await.ok()?;
    security
        .split(str)
        .nth(1)?
        .split(" ;  path=/")
        .next()
        .map(|s| str.to_owned() + s.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    use reqwest::Url;

    pub fn get_url(raw_url: &str) -> String {
        Url::parse(raw_url)
            .unwrap()
            .path_segments()
            .and_then(|mut s| s.next_back())
            .unwrap()
            .into()
    }

    #[tokio::test]
    async fn test_find_cookie() {
        let text = r#"<html><body><script>document.cookie="SecurityAW-E4=ccf64e38a09ed38849d9ae72e1931e5b ;  path=/";location.href="http://www.animeworld.so/?d=1";</script></body></html>"#;

        let res = text
            .split("SecurityAW")
            .nth(1)
            .unwrap()
            .split("path=/")
            .next()
            .map(|s| "SecurityAW".to_owned() + s.trim())
            .unwrap();

        assert_eq!(res, "SecurityAW-E4=ccf64e38a09ed38849d9ae72e1931e5b ;")
    }

    #[tokio::test]
    #[ignore]
    async fn test_remote_scraper() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";

        let site = Site::AW;
        let proxy = select_proxy(false).await;
        let cookie = find_cookie(Site::AW).await;
        let search = vec![Search {
            string: "bunny girl".into(),
            id: None,
        }];

        let (anime, _) = Scraper::new(proxy, cookie)
            .run(search.into_iter(), site)
            .await
            .unwrap();
        let info = get_url(&anime.first().unwrap().origin);

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
        let cookie = find_cookie(Site::AW).await;

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
                string: "promare (ita)".into(),
                id: None,
            },
        ];

        let (anime, _) = Scraper::new(proxy, cookie)
            .run(search.into_iter(), site)
            .await
            .unwrap();

        let mut anime = anime
            .iter()
            .map(|a| {
                Url::parse(&a.origin)
                    .unwrap()
                    .path_segments()
                    .and_then(|mut s| s.next_back())
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        anime.sort();
        files.sort_unstable();

        assert_eq!(anime, files)
    }
}
