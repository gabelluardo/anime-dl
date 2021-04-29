use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

// pub use anyhow::{bail, Result};
use futures::future::join_all;
use rand::seq::IteratorRandom;
use reqwest::{header, header::HeaderValue, Client, Url};
use scraper::{Html, Selector};
use tokio::sync::Mutex;

use crate::cli::Site;
use crate::errors::{Error, Result};
use crate::utils::tui;

#[derive(Debug, Clone)]
pub struct ScraperItem {
    pub id: Option<u32>,
    pub url: String,
}

#[derive(Debug, Default, Clone)]
pub struct ScraperCollector {
    pub items: Vec<ScraperItem>,
    pub referer: String,
}

impl ScraperCollector {
    fn new() -> Self {
        Self::default()
    }

    fn mutex() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }
}

impl Deref for ScraperCollector {
    type Target = Vec<ScraperItem>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for ScraperCollector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl FromIterator<ScraperItem> for ScraperCollector {
    fn from_iter<I: IntoIterator<Item = ScraperItem>>(iter: I) -> Self {
        let mut c = ScraperCollector::new();
        c.extend(iter);
        c
    }
}

#[derive(Default)]
pub struct Scraper {
    proxy: bool,
    query: String,
    site: Option<Site>,
}

impl Scraper {
    pub fn new(proxy: bool, query: &str, site: Option<Site>) -> Self {
        Self {
            proxy,
            site,
            query: query.to_string(),
        }
    }

    pub fn _proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn _query(mut self, query: &str) -> Self {
        self.query = query.to_string();
        self
    }

    pub fn _site(mut self, site: Option<Site>) -> Self {
        self.site = site;
        self
    }

    pub fn _collector() -> ScraperCollector {
        ScraperCollector::new()
    }

    pub fn item(url: &str, id: Option<u32>) -> ScraperItem {
        ScraperItem {
            id,
            url: url.to_owned(),
        }
    }

    pub async fn run(self) -> Result<ScraperCollector> {
        let query = self
            .query
            .split(',')
            .map(|s| s.trim().replace(" ", "+"))
            .collect::<Vec<_>>();

        let func = match self.site {
            Some(Site::AW) | None => Self::animeworld,
        };

        let sc = ScraperCollector::mutex();
        let tasks = query
            .iter()
            .map(|q| func(q, self.proxy, sc.clone()))
            .map(|f| async move { ok!(f.await) })
            .collect::<Vec<_>>();

        join_all(tasks).await;

        let res = sc.lock().await.clone();

        Ok(res)
    }

    async fn animeworld(query: &str, proxy: bool, buf: Arc<Mutex<ScraperCollector>>) -> Result<()> {
        let client = ScraperClient::new(proxy).await?;
        let search_url = format!("https://www.animeworld.tv/search?keyword={}", query);

        let page = Self::parse(search_url, &client).await?;
        let results = {
            let div = Selector::parse("div.film-list").unwrap();
            let a = Selector::parse("a.name").unwrap();

            let elem = page
                .select(&div)
                .next()
                .ok_or(Error::with_msg("Request blocked, retry"))?;
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
            bail!(Error::AnimeNotFound)
        }

        let choices = tui::get_choice(results, Some(query.replace("+", " "))).await?;

        let pages = choices
            .iter()
            .map(|c| Self::parse(format!("https://www.animeworld.tv{}", c), &client))
            .collect::<Vec<_>>();

        let res = join_all(pages)
            .await
            .into_iter()
            .filter_map(|p| p.ok())
            .map(|page| {
                let a = Selector::parse(r#"a[id="alternativeDownloadLink"]"#).unwrap();
                let btn = Selector::parse(r#"a[id="anilist-button"]"#).unwrap();

                let url = page.select(&a).last().and_then(|a| a.value().attr("href"));
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

                Self::item(url.unwrap_or_default(), id)
            })
            .filter(|i| !i.url.is_empty())
            .collect::<Vec<_>>();

        if res.is_empty() {
            bail!(Error::UrlNotFound)
        }

        let mut buf = buf.lock().await;
        buf.extend(res);

        if buf.referer.is_empty() {
            buf.referer = "https://www.animeworld.tv/".to_string();
        }

        Ok(())
    }

    async fn parse(url: String, client: &Client) -> Result<Html> {
        let response = client.get(&url).send().await?.error_for_status()?;

        Ok(Html::parse_document(&response.text().await?))
    }
}

struct ScraperClient(Client);

impl<'a> ScraperClient {
    #[rustfmt::skip]
    const ACCEPT: &'a str = "text/html,application/xhtml+xml,application/xml; q=0.9,image/webp,*/*; q=0.8";
    const COOKIES: &'a str = "__cfduid=d03255bed084571c421edd313dbfd5fe31610142561; _csrf=PLwPaldqI-hCpuZzS8wfLnkP; expandedPlayer=false; theme=dark";
    const USER_AGENT: &'a str = "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-GB; rv:1.8.1.6) Gecko/20070725 Firefox/2.0.0.6";

    async fn new(proxy: bool) -> Result<Self> {
        let mut builder = Client::builder()
            .user_agent(Self::USER_AGENT)
            .default_headers(Self::set_headers());

        if proxy {
            builder = builder.proxy(Self::set_proxy().await?);
        }

        let client = builder.build()?;

        Ok(Self(client))
    }

    async fn set_proxy() -> Result<reqwest::Proxy> {
        let response = reqwest::get("https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=2000&country=all&ssl=all&anonymity=elite")
            .await?
            .text()
            .await?;

        let proxy = response
            .split_ascii_whitespace()
            .choose(&mut rand::thread_rng())
            .map(|s| format!("https://{}", s));

        reqwest::Proxy::http(&proxy.unwrap()).map_err(|_| Error::Proxy)
    }

    fn set_headers() -> header::HeaderMap {
        let mut headers = header::HeaderMap::new();

        headers.insert(header::COOKIE, HeaderValue::from_static(Self::COOKIES));
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));

        headers
    }
}

impl Deref for ScraperClient {
    type Target = Client;

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
        let proxy_client = ScraperClient::new(false).await;
        let no_proxy_client = ScraperClient::new(true).await;

        proxy_client.unwrap();
        no_proxy_client.unwrap();
    }

    #[tokio::test]
    async fn test_animeworld() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = ScraperCollector::mutex();

        Scraper::animeworld("bunny girl", false, anime.clone())
            .await
            .unwrap();
        let anime = anime.lock().await.clone();

        let info = get_url(&anime.first().unwrap().url);

        assert_eq!(file, info)
    }

    #[tokio::test]
    async fn test_scraper() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Scraper::default()
            ._site(Some(Site::AW))
            ._query("bunny girl")
            .run()
            .await
            .unwrap();

        let info = get_url(&anime.first().unwrap().url);

        assert_eq!(file, info)
    }

    #[tokio::test]
    async fn test_scraper_multi() {
        let mut files = vec![
            "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4",
            "TsurezureChildren_Ep_01_SUB_ITA.mp4",
            "Promare_Movie_ITA.mp4",
        ];

        let anime = Scraper::default()
            ._site(Some(Site::AW))
            ._query("bunny girl, tsuredure children, promare")
            .run()
            .await
            .unwrap();

        let mut anime = anime
            .iter()
            .map(|a| {
                Url::parse(&a.url)
                    .unwrap()
                    .path_segments()
                    .and_then(|segments| segments.last())
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        assert_eq!(anime.sort(), files.sort())
    }
}
