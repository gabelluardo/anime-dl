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
        let Some(chosen) = list.split_ascii_whitespace().choose(&mut rand::rng()) else {
            return Err(anyhow::anyhow!("No proxy found"));
        };

        let proxy = format!("https://{chosen}");

        Ok(proxy)
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

    async fn scraper_single<T: Archive>(search_query: &str, expected_file: &str) -> Result<()> {
        let cookie = T::extract_cookie().await;
        let config = ScraperConfig {
            cookie,
            proxy: None,
        };
        let search = vec![Search {
            string: search_query.into(),
            id: None,
        }];

        let anime = Scraper::new(config).search::<T>(&search).await?;
        let info = get_url(
            &anime
                .first()
                .ok_or_else(|| anyhow::anyhow!("No anime found"))?
                .origin,
        );

        assert_eq!(expected_file, info);
        Ok(())
    }

    async fn scraper_multi<T: Archive>(
        search_queries: Vec<&str>,
        expected_files: Vec<&str>,
    ) -> Result<()> {
        let mut expected = expected_files;
        expected.sort_unstable();

        let cookie = T::extract_cookie().await;
        let config = ScraperConfig {
            cookie,
            proxy: None,
        };

        let search: Vec<_> = search_queries
            .into_iter()
            .map(|s| Search {
                string: s.into(),
                id: None,
            })
            .collect();

        let anime = Scraper::new(config).search::<T>(&search).await?;

        let mut results: Vec<_> = anime
            .iter()
            .map(|a| {
                Url::parse(&a.origin)
                    .unwrap()
                    .path_segments()
                    .and_then(|mut s| s.next_back())
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            })
            .collect();

        results.sort();
        assert_eq!(results, expected);
        Ok(())
    }

    macro_rules! test_archive_scraper {
        ($($archive_type:ty),+ $(,)?) => {
            $(
                paste::paste! {
                    #[tokio::test]
                    #[ignore]
                    async fn [<test_ $archive_type:lower >]() {
                        scraper_single::<$archive_type>(
                            "bunny girl",
                            "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4",
                        )
                        .await
                        .unwrap()
                    }

                    #[tokio::test]
                    #[ignore]
                    async fn [<test_ $archive_type:lower _multi>]() {
                        scraper_multi::<$archive_type>(
                            vec!["bunny girl", "tsuredure children", "promare (ita)"],
                            vec![
                                "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4",
                                "TsurezureChildren_Ep_01_SUB_ITA.mp4",
                                "Promare_Movie_ITA.mp4",
                            ],
                        )
                        .await
                        .unwrap()
                    }
                }
            )+
        };
    }

    // Add each new archive here: test_archive_scraper!(AnimeWorld, MyNewArchive);
    test_archive_scraper!(AnimeWorld);
}
