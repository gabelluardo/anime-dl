use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use futures::future::join_all;
use owo_colors::OwoColorize;
use rand::seq::IteratorRandom;
use reqwest::{header, header::HeaderValue, Client as RClient, Url};
use scraper::{Html, Selector};
use tokio::sync::Mutex;

use crate::anime::AnimeInfo;
use crate::cli::Site;
use crate::errors::{Quit, RemoteError};
use crate::utils::{self, tui};

#[derive(Debug, Default, Clone)]
pub struct ScraperCollector {
    pub items: Vec<AnimeInfo>,
    pub referrer: String,
}

impl Deref for ScraperCollector {
    type Target = Vec<AnimeInfo>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for ScraperCollector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl FromIterator<AnimeInfo> for ScraperCollector {
    fn from_iter<I: IntoIterator<Item = AnimeInfo>>(iter: I) -> Self {
        let mut c = ScraperCollector::default();
        c.extend(iter);
        c
    }
}

struct Archive;

impl Archive {
    async fn animeworld(param: (&str, Arc<Client>, Arc<Mutex<ScraperCollector>>)) -> Result<()> {
        let (query, client, buf) = param;

        let search_url = format!("https://www.animeworld.tv/search?keyword={query}");

        let page = client.parse_url(&search_url).await?;
        let results = {
            let div = Selector::parse("div.film-list").unwrap();
            let a = Selector::parse("a.name").unwrap();

            let elem = page.select(&div).next().context("Request blocked, retry")?;
            elem.select(&a)
                .into_iter()
                .map(|a| {
                    let link = a.value().attr("href").expect("No link found").to_string();
                    let name = a
                        .first_child()
                        .and_then(|a| a.value().as_text())
                        .expect("No name found")
                        .to_string();

                    tui::Choice::new(link, name)
                })
                .collect::<Vec<_>>()
        };

        if results.is_empty() {
            bail!(RemoteError::AnimeNotFound)
        }

        let choices = tui::get_choice(&results, Some(query.replace('+', " ")))?;

        let choices = choices
            .iter()
            .map(|c| format!("https://www.animeworld.tv{c}"))
            .collect::<Vec<_>>();

        let pages = choices
            .iter()
            .map(|u| client.parse_url(u))
            .collect::<Vec<_>>();

        let res = join_all(pages)
            .await
            .into_iter()
            .filter_map(|p| p.ok())
            .map(|page| {
                let a = Selector::parse(r#"a[id="alternativeDownloadLink"]"#).unwrap();
                let mut url = page
                    .select(&a)
                    .last()
                    .and_then(|a| a.value().attr("href"))
                    .map(|u| u.to_string());

                // try again with another links
                if url.is_none() || url == Some("".to_string()) {
                    let a = Selector::parse(r#"a[id="downloadLink"]"#).unwrap();

                    url = page
                        .select(&a)
                        .last()
                        .and_then(|a| a.value().attr("href"))
                        .map(|u| u.replace("download-file.php?id=", ""));
                }

                if url.is_none() || url == Some("".to_string()) {
                    let a = Selector::parse(r#"a[id="customDownloadButton"]"#).unwrap();

                    url = page
                        .select(&a)
                        .last()
                        .and_then(|a| a.value().attr("href"))
                        .map(|u| u.replace("download-file.php?id=", ""));
                }

                let btn = Selector::parse(r#"a[id="anilist-button"]"#).unwrap();
                let id = page
                    .select(&btn)
                    .last()
                    .and_then(|a| a.value().attr("href"))
                    .and_then(|u| {
                        Url::parse(u)
                            .unwrap()
                            .path_segments()
                            .and_then(|s| s.last())
                            .and_then(|s| s.parse::<u32>().ok())
                    });

                let url = url.unwrap_or_default();

                AnimeInfo::new(&url, id).unwrap_or_default()
            })
            .filter(|info| !info.url.is_empty())
            .collect::<Vec<_>>();

        if res.is_empty() {
            bail!(RemoteError::UrlNotFound)
        }

        let mut buf = buf.lock().await;
        buf.extend(res);

        if buf.referrer.is_empty() {
            buf.referrer = "https://www.animeworld.tv/".to_string();
        }

        Ok(())
    }

    async fn _placeholder(_param: (&str, Arc<Client>, Arc<Mutex<ScraperCollector>>)) -> Result<()> {
        unimplemented!()
    }
}

#[derive(Debug, Default)]
pub struct Scraper {
    proxy: bool,
    query: String,
    site: Site,
}

impl Scraper {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            ..Self::default()
        }
    }

    pub fn with_proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }

    pub async fn run(self) -> Result<ScraperCollector> {
        let query = self
            .query
            .split(',')
            .map(|s| s.trim().replace(' ', "+"))
            .collect::<Vec<_>>();

        let mut proxy = None;
        if self.proxy {
            proxy = Client::find_proxy().await.ok();
        }

        let ctest = Client::find_ctest().await?;
        let client = Arc::new(Client::new(proxy, &ctest));

        let sc = Arc::new(Mutex::new(ScraperCollector::default()));
        let tasks = query
            .iter()
            .map(|q| async {
                let param = (q.as_str(), client.clone(), sc.clone());

                match self.site {
                    Site::AW => Archive::animeworld(param).await,
                    // _ => Archive::placeholder(param).await,
                }
            })
            .map(|f| async move {
                if let Err(err) = f.await {
                    if !err.is::<Quit>() {
                        eprintln!("{}", err.red());
                    }
                }
            })
            .collect::<Vec<_>>();

        join_all(tasks).await;

        let res = sc.lock().await.clone();

        Ok(res)
    }
}

#[derive(Default, Debug)]
pub struct Client(RClient);

impl<'a> Client {
    const ACCEPT: &'a str =
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8";
    const COOKIE: &'a str = "__ddg1=sti44Eo5SrS4IAwJPVFu; __cfduid=d1343ee68e09afafe0a4855d5c35e713f1619342282; _csrf=wSnjNmhifYyOPULeghB6Dloy;";
    const PROXY_SCRAPE: &'a str = "https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=2000&country=all&ssl=all&anonymity=elite";
    const USER_AGENT: &'a str =
        "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";

    fn new(proxy: Option<String>, ctest: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        let cookie = HeaderValue::from_str(ctest).unwrap();

        headers.insert(header::COOKIE, cookie);
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static(Self::USER_AGENT),
        );

        let mut builder = RClient::builder().default_headers(headers);

        if let Some(proxy) = proxy {
            if let Ok(req_proxy) = reqwest::Proxy::http(proxy).context(RemoteError::Proxy) {
                builder = builder.proxy(req_proxy)
            }
        }

        let client = builder.build().unwrap_or_default();

        Self(client)
    }

    async fn find_proxy() -> Result<String> {
        let res = reqwest::get(Self::PROXY_SCRAPE).await?.text().await?;

        let proxy = res
            .split_ascii_whitespace()
            .choose(&mut rand::thread_rng())
            .map(|s| format!("https://{s}"))
            .unwrap_or_default();

        Ok(proxy)
    }

    async fn find_ctest() -> Result<String> {
        let text = reqwest::get("https://www.animeworld.tv/")
            .await?
            .text()
            .await?;
        let mut ctest = utils::parse_aw_cookie(&text).unwrap_or_default();
        ctest.push_str(Self::COOKIE);

        Ok(ctest)
    }

    async fn parse_url(&self, url: &str) -> Result<Html> {
        let response = self.get(url).send().await?.error_for_status()?;

        Ok(Html::parse_fragment(&response.text().await?))
    }
}

impl Deref for Client {
    type Target = RClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Url;

    use super::*;

    fn get_url(raw_url: &str) -> String {
        Url::parse(raw_url)
            .unwrap()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .to_owned()
    }

    #[tokio::test]
    async fn test_client() {
        let local_proxy = Some("127.0.0.1".to_string());

        Client::new(local_proxy, "");
        Client::new(None, "");

        let proxy = Client::find_proxy().await.ok();
        let ctest = Client::find_ctest().await.unwrap();

        Client::new(proxy, &ctest);
        Client::new(None, &ctest);
    }

    #[tokio::test]
    #[ignore]
    async fn test_animeworld() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Arc::new(Mutex::new(ScraperCollector::default()));
        let client = Arc::new(Client::default());

        let param = ("bunny girl", client, anime.clone());
        Archive::animeworld(param).await.unwrap();

        let anime = anime.lock().await.clone();
        let info = get_url(&anime.first().unwrap().origin);

        assert_eq!(file, info)
    }

    #[tokio::test]
    #[ignore]
    async fn test_scraper() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Scraper::new("bunny girl")
            .with_proxy(true)
            .run()
            .await
            .unwrap();

        let info = get_url(&anime.first().unwrap().origin);

        assert_eq!(file, info)
    }

    #[tokio::test]
    #[ignore]
    async fn test_scraper_multi() {
        let mut files = vec![
            "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4",
            "TsurezureChildren_Ep_01_SUB_ITA.mp4",
            "Promare_Movie_ITA.mp4",
        ];

        let anime = Scraper::new("bunny girl, tsuredure children, promare")
            .with_proxy(true)
            .run()
            .await
            .unwrap();

        let mut anime = anime
            .iter()
            .map(|a| {
                Url::parse(&a.origin)
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
