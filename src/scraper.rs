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
use crate::tui;
use crate::utils;

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

struct Archive;

impl Archive {
    async fn animeworld(param: (&str, Arc<Client>, Arc<Mutex<ScraperItems>>)) -> Result<()> {
        async fn inner(client: Arc<Client>, url: String) -> Result<AnimeInfo> {
            let page = client.parse_url(&url).await?;
            let a = Selector::parse(r#"a[id="alternativeDownloadLink"]"#).unwrap();
            let mut url = page.select(&a).last().and_then(|a| a.value().attr("href"));
            // try again with other links
            if url.is_none() || url == Some("") {
                let a = Selector::parse(r#"a[id="downloadLink"]"#).unwrap();
                url = page.select(&a).last().and_then(|a| a.value().attr("href"))
            }
            if url.is_none() || url == Some("") {
                let a = Selector::parse(r#"a[id="customDownloadButton"]"#).unwrap();
                url = page.select(&a).last().and_then(|a| a.value().attr("href"))
            }
            url.map(|u| u.replace("download-file.php?id=", ""));
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
            AnimeInfo::new(url.unwrap_or_default(), id)
        }

        let (query, client, buf) = param;
        let search_results = {
            let search_url = format!("https://www.animeworld.tv/search?keyword={query}");
            let search_page = client.parse_url(&search_url).await?;
            let anime_list: Selector = Selector::parse("div.film-list").unwrap();
            let name = Selector::parse("a.name").unwrap();
            let elem = search_page
                .select(&anime_list)
                .next()
                .context("Request blocked, retry")?;
            elem.select(&name)
                .map(|a| {
                    let link = a.value().attr("href").expect("No link found");
                    let name = a
                        .first_child()
                        .and_then(|a| a.value().as_text())
                        .expect("No name found");
                    tui::Choice::new(link, name)
                })
                .collect::<Vec<_>>()
        };
        if search_results.is_empty() {
            bail!(RemoteError::AnimeNotFound)
        }
        let selected = tui::get_choice(&search_results, Some(query.replace('+', " ")))?;
        let mut req = vec![];
        for c in selected {
            let url = format!("https://www.animeworld.tv{c}");
            req.push(inner(client.clone(), url))
        }
        let res = join_all(req)
            .await
            .into_iter()
            .filter_map(|a| a.ok())
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

    async fn _placeholder(_param: (&str, Arc<Client>, Arc<Mutex<ScraperItems>>)) -> Result<()> {
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

    async fn choice_archive(
        &self,
        param: (&str, Arc<Client>, Arc<Mutex<ScraperItems>>),
    ) -> Result<()> {
        match self.site {
            Site::AW => Archive::animeworld(param).await,
            // _ => Archive::_placeholder(param).await,
        }
    }

    pub async fn run(self) -> Result<ScraperItems> {
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
        let sc = Arc::new(Mutex::new(ScraperItems::default()));
        let tasks = query
            .iter()
            .map(|q| self.choice_archive((q, client.clone(), sc.clone())))
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
    async fn test_remote_animeworld() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Arc::new(Mutex::new(ScraperItems::default()));
        let client = Arc::new(Client::default());

        let param = ("bunny girl", client, anime.clone());
        Archive::animeworld(param).await.unwrap();

        let anime = anime.lock().await.clone();
        let info = get_url(&anime.first().unwrap().origin);

        assert_eq!(file, info)
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
