use std::time::Duration;

use anyhow::Result;
use futures::future::join_all;
use owo_colors::OwoColorize;
use rand::seq::IteratorRandom;
use reqwest::{Client, header, header::HeaderValue};

use crate::anime::Anime;
use crate::archive::Archive;

#[derive(Debug, Clone)]
pub struct Search {
    pub id: Option<u32>,
    pub string: String,
}

#[derive(Debug)]
pub struct ScraperConfig {
    pub cookie: Option<String>,
    pub proxy: Option<String>,
}

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct Scraper(Client);

impl Scraper {
    pub fn new(config: ScraperConfig) -> Self {
        let ScraperConfig { cookie, proxy } = config;

        let mut headers = header::HeaderMap::new();
        if let Some(c) = cookie
            && let Ok(value) = HeaderValue::from_str(&c)
        {
            headers.insert(header::COOKIE, value);
        }

        let mut builder = Client::builder()
            .default_headers(headers)
            .connect_timeout(CONNECTION_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            // this enables support for http only sites
            .danger_accept_invalid_certs(true);

        if let Some(p) = proxy
            && let Ok(req_proxy) = reqwest::Proxy::http(p)
        {
            builder = builder.proxy(req_proxy)
        }

        let client = builder.build().unwrap_or_default();

        Self(client)
    }

    pub async fn search<T: Archive>(&self, searches: &[Search]) -> Result<Vec<Anime>> {
        let tasks = searches.iter().map(|search| {
            let client = self.0.clone();
            let search = search.clone();

            async move {
                T::search(search, client).await.unwrap_or_else(|err| {
                    eprintln!("{}", err.red());
                    vec![]
                })
            }
        });

        let anime = join_all(tasks).await.into_iter().flatten().collect();

        Ok(anime)
    }

    #[cfg(test)]
    pub fn client(&self) -> Client {
        self.0.clone()
    }
}

pub struct ProxyManager;

impl ProxyManager {
    pub async fn proxy(disable: bool) -> Option<String> {
        if disable {
            return None;
        }

        Self::get_random_proxy().await.ok()
    }

    async fn get_random_proxy() -> Result<String> {
        let url = "https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=2000&country=all&ssl=all&anonymity=elite";
        let list = reqwest::get(url).await?.text().await?;

        let proxy = list
            .split_ascii_whitespace()
            .choose(&mut rand::rng())
            .map(|s| format!("https://{s}"))
            .ok_or_else(|| anyhow::anyhow!("No proxy found"))?;

        Ok(proxy)
    }
}

pub struct CookieManager;

impl CookieManager {
    pub async fn extract_cookie_for_site<T: Archive>() -> Option<String> {
        Self::extract_cookie_from_url(T::REFERRER, T::COOKIE_NAME)
            .await
            .ok()
            .flatten()
    }

    async fn extract_cookie_from_url(url: &str, cookie_name: &str) -> Result<Option<String>> {
        let response = reqwest::get(url).await?.text().await?;

        let cookie = response
            .split(cookie_name)
            .nth(1)
            .and_then(|s| s.split(" ;  path=/").next())
            .map(|s| cookie_name.to_owned() + s.trim());

        Ok(cookie)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::archive::AnimeWorld;

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

        let cookie = CookieManager::extract_cookie_for_site::<AnimeWorld>().await;
        let config = ScraperConfig {
            cookie,
            proxy: None,
        };
        let search = vec![Search {
            string: "bunny girl".into(),
            id: None,
        }];

        let anime = Scraper::new(config)
            .search::<AnimeWorld>(&search)
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

        let cookie = CookieManager::extract_cookie_for_site::<AnimeWorld>().await;
        let config = ScraperConfig {
            cookie,
            proxy: None,
        };

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

        let anime = Scraper::new(config)
            .search::<AnimeWorld>(&search)
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
