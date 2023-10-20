use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use anyhow::{Context, Result};
use futures::future::join_all;
use owo_colors::OwoColorize;
use rand::seq::IteratorRandom;
use reqwest::{header, header::HeaderValue, Client as RClient};
use scraper::Html;
use tokio::sync::Mutex;

use crate::anime::AnimeInfo;
use crate::archive::{AnimeWorld, Archive};
use crate::cli::Site;
use crate::errors::{Quit, RemoteError};
use crate::parser;

#[derive(Debug, Default, Clone)]
pub struct ScraperItems {
    pub items: Vec<AnimeInfo>,
    pub referrer: String,
}

impl Deref for ScraperItems {
    type Target = Vec<AnimeInfo>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for ScraperItems {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl FromIterator<AnimeInfo> for ScraperItems {
    fn from_iter<I: IntoIterator<Item = AnimeInfo>>(iter: I) -> Self {
        let mut c = ScraperItems::default();
        c.extend(iter);
        c
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
            query: query.into(),
            ..Self::default()
        }
    }

    pub fn with_proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }

    pub async fn run(self) -> Result<ScraperItems> {
        let (scraper_fun, referrer) = match self.site {
            Site::AW => (AnimeWorld::run, AnimeWorld::referrer()),
            // _ => Placeholder::run,
        };

        let query = self
            .query
            .split(',')
            .map(|s| s.trim().replace(' ', "+"))
            .collect::<Vec<_>>();
        let mut proxy = None;
        if self.proxy {
            proxy = Client::find_proxy().await.ok();
        }

        let ctest = Client::find_ctest(self.site).await?;
        let client = Arc::new(Client::new(proxy, &ctest));
        let vec = Arc::new(Mutex::new(Vec::new()));
        let tasks = query
            .iter()
            .map(|q| scraper_fun(q, client.clone(), vec.clone()))
            .map(|f| async move {
                if let Err(err) = f.await {
                    if !err.is::<Quit>() {
                        eprintln!("{}", err.red());
                    }
                }
            })
            .collect::<Vec<_>>();
        join_all(tasks).await;

        Ok(ScraperItems {
            items: vec.lock_owned().await.to_vec(),
            referrer: referrer.unwrap_or_default(),
        })
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

    async fn find_ctest(site: Site) -> Result<String> {
        let referrer = match site {
            Site::AW => AnimeWorld::referrer(),
        };

        let mut ctest = String::new();
        if let Some(url) = referrer {
            let text = reqwest::get(url).await?.text().await?;
            ctest = parser::parse_aw_cookie(&text).unwrap_or_default();
            ctest.push_str(Self::COOKIE);
        }

        Ok(ctest)
    }

    pub async fn parse_url(&self, url: &str) -> Result<Html> {
        let response = self.get(url).send().await?.error_for_status()?;
        let fragment = Html::parse_fragment(&response.text().await?);
        Ok(fragment)
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
    use super::*;
    use reqwest::Url;

    pub fn get_url(raw_url: &str) -> String {
        Url::parse(raw_url)
            .unwrap()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .to_owned()
    }

    #[tokio::test]
    async fn test_client() {
        let local_proxy = Some("127.0.0.1".into());

        Client::new(local_proxy, "");
        Client::new(None, "");

        let proxy = Client::find_proxy().await.ok();
        let ctest = Client::find_ctest(Site::AW).await.unwrap();

        Client::new(proxy, &ctest);
        Client::new(None, &ctest);
    }

    #[tokio::test]
    #[ignore]
    async fn test_remote_scraper() {
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
    async fn test_remote_scraper_multi() {
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
