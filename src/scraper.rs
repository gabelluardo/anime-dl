use std::time::Duration;

use anyhow::Result;
use futures::future::join_all;
use owo_colors::OwoColorize;
use reqwest::{Client, header, header::HeaderValue};

use crate::{
    anilist::AnilistId,
    anime::{Anime, AnimeId},
    archives::Archive,
};

#[derive(Debug, Clone)]
pub struct Search {
    pub id: Option<AnimeId>,
    pub string: String,
}

impl Search {
    pub fn new(string: impl Into<String>, id: Option<AnimeId>) -> Self {
        Self {
            id,
            string: string.into(),
        }
    }
}

#[derive(Debug)]
pub struct ScraperConfig {
    pub cookie: Option<String>,
    pub proxy: Option<String>,
    pub anilist_id: Option<AnilistId>,
}

#[derive(Debug)]
pub struct Scraper {
    client: Client,
    anilist_id: Option<AnilistId>,
}

impl Scraper {
    const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn new(config: ScraperConfig) -> Self {
        let ScraperConfig {
            cookie,
            proxy,
            anilist_id,
        } = config;

        let mut headers = header::HeaderMap::new();
        if let Some(c) = cookie
            && let Ok(value) = HeaderValue::from_str(&c)
        {
            headers.insert(header::COOKIE, value);
        }

        let mut builder = Client::builder()
            .default_headers(headers)
            .connect_timeout(Self::CONNECTION_TIMEOUT)
            .timeout(Self::REQUEST_TIMEOUT)
            // this enables support for http only sites
            .danger_accept_invalid_certs(true);

        if let Some(p) = proxy
            && let Ok(req_proxy) = reqwest::Proxy::http(p)
        {
            builder = builder.proxy(req_proxy)
        }

        let client = builder.build().unwrap_or_default();

        Self { client, anilist_id }
    }

    pub async fn search<T: Archive>(&self, searches: &[Search]) -> Result<Vec<Anime>> {
        let anilist_id = self.anilist_id;
        let tasks = searches.iter().map(|search| {
            let client = self.client.clone();
            let search = search.clone();

            async move {
                T::search(search, client, anilist_id)
                    .await
                    .unwrap_or_else(|err| {
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
        self.client.clone()
    }
}

pub mod selector {
    use anyhow::Result;
    use reqwest::Client;
    use scraper::{Html, Selector};

    pub fn from(selectors: &str) -> Selector {
        let Ok(s) = Selector::parse(selectors) else {
            panic!("unable to parse selector {selectors}")
        };

        s
    }

    pub async fn get_page(client: &Client, url: &str) -> Result<Html> {
        let response = client.get(url).send().await?.error_for_status()?;
        let fragment = Html::parse_fragment(&response.text().await?);

        Ok(fragment)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use simple_test_case::test_case;

        #[test_case("div"; "simple tag")]
        #[test_case("div.class"; "class selector")]
        #[test_case("a[href]"; "attribute selector")]
        #[test_case("#id"; "id selector")]
        #[test]
        fn test_from_valid(selectors: &str) {
            let s = from(selectors);
            // just verify it doesn't panic
            let _ = format!("{s:?}");
        }

        #[test]
        #[should_panic(expected = "unable to parse selector")]
        fn test_from_invalid() {
            let _ = from(">>>");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::archives::AnimeWorld;
    use reqwest::Url;
    use simple_test_case::test_case;

    pub fn get_url(raw_url: &str) -> String {
        Url::parse(raw_url)
            .unwrap()
            .path_segments()
            .and_then(|mut s| s.next_back())
            .unwrap()
            .into()
    }

    #[test_case(
        r#"<html><body><script>document.cookie="SecurityAW-E4=ccf64e38a09ed38849d9ae72e1931e5b ;  path=/";location.href="http://www.animeworld.so/?d=1";</script></body></html>"#,
        "SecurityAW-E4=ccf64e38a09ed38849d9ae72e1931e5b ;";
        "animeworld cookie"
    )]
    #[test_case(
        r#"<script>document.cookie="SecurityAW-XY=session-token ; path=/";</script>"#,
        "SecurityAW-XY=session-token ;";
        "trim extra spaces before path"
    )]
    #[test]
    fn test_find_cookie(text: &str, expected: &str) {
        let res = text
            .split("SecurityAW")
            .nth(1)
            .unwrap()
            .split("path=/")
            .next()
            .map(|s| "SecurityAW".to_owned() + s.trim())
            .unwrap();

        assert_eq!(res, expected)
    }

    #[test_case("bunny girl", None; "search without id")]
    #[test_case("bunny girl", Some(AnimeId(42)); "search with id")]
    #[test_case("", None; "empty search string")]
    #[test]
    fn test_search_new(string: &str, id: Option<AnimeId>) {
        let search = Search::new(string, id);
        assert_eq!(search.string, string);
        assert_eq!(search.id, id);
    }

    #[test_case("test", None; "owned string")]
    #[test_case("bunny girl", None; "string with space")]
    #[test_case("promare+ita", None; "string with plus")]
    #[test_case("", None; "empty string")]
    #[test_case("test", Some(AnimeId(42)); "with id")]
    #[test]
    fn test_search_new_various(string: &str, id: Option<AnimeId>) {
        let search = Search::new(string, id);
        assert_eq!(search.string, string);
        assert_eq!(search.id, id);
    }

    fn build_config(
        cookie: Option<&str>,
        proxy: Option<&str>,
        anilist_id: Option<u32>,
    ) -> ScraperConfig {
        ScraperConfig {
            cookie: cookie.map(String::from),
            proxy: proxy.map(String::from),
            anilist_id: anilist_id.map(AnilistId::from),
        }
    }

    #[test_case(None, None, None; "all none")]
    #[test_case(Some("test=cookie"), None, None; "with cookie")]
    #[test_case(Some("\ninvalid"), None, None; "invalid cookie")]
    #[test_case(None, None, Some(42); "with anilist id")]
    #[test_case(None, Some("not a url"), None; "invalid proxy")]
    #[test_case(None, Some("http://proxy.example.com:8080"), None; "valid proxy")]
    #[test_case(Some("test=cookie"), Some("http://proxy.example.com:8080"), Some(42); "all set")]
    #[test]
    fn test_scraper_new(cookie: Option<&str>, proxy: Option<&str>, anilist_id: Option<u32>) {
        let config = build_config(cookie, proxy, anilist_id);
        let scraper = Scraper::new(config);
        let _ = scraper.client();
    }

    #[tokio::test]
    async fn test_scraper_search_empty() {
        let config = ScraperConfig {
            cookie: None,
            proxy: None,
            anilist_id: None,
        };
        let scraper = Scraper::new(config);
        let result = scraper.search::<AnimeWorld>(&[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[test_case("test", Some(AnimeId(42)); "with id")]
    #[test_case("test", None; "without id")]
    #[test_case("", None; "empty string")]
    #[test]
    fn test_search_clone(string: &str, id: Option<AnimeId>) {
        let search = Search::new(string, id);
        let cloned = search.clone();
        assert_eq!(search.string, cloned.string);
        assert_eq!(search.id, cloned.id);
    }

    async fn scraper_single<T: Archive>(search_query: &str, expected_file: &str) -> Result<()> {
        let cookie = T::extract_cookie().await;
        let config = ScraperConfig {
            cookie,
            proxy: None,
            anilist_id: None,
        };
        let search = vec![Search::new(search_query, None)];

        let anime = Scraper::new(config).search::<T>(&search).await?;
        let info = get_url(anime.first().unwrap().url());

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
            anilist_id: None,
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
                Url::parse(a.url())
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
