use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use futures::future::join_all;
use owo_colors::OwoColorize;
use rand::seq::IteratorRandom;
use reqwest::{header, header::HeaderValue, Client as RClient, Url};
use scraper::{Html, Selector};
use tokio::sync::Mutex;

use crate::anime::AnimeInfo;
use crate::cli::Site;
use crate::errors::{Error, Result};
use crate::utils::{tui, Info};

#[derive(Debug, Default, Clone)]
pub struct ScraperCollector {
    pub items: Vec<AnimeInfo>,
    pub referrer: String,
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
        let mut c = ScraperCollector::new();
        c.extend(iter);
        c
    }
}

pub struct Scraper {
    proxy: bool,
    query: String,
    site: Site,
}

impl Default for Scraper {
    fn default() -> Self {
        Self {
            proxy: true,
            query: String::default(),
            site: Site::default(),
        }
    }
}

impl Scraper {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            ..Self::default()
        }
    }

    pub fn proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn _site(mut self, site: Site) -> Self {
        self.site = site;
        self
    }

    pub fn _collector() -> ScraperCollector {
        ScraperCollector::new()
    }

    pub fn item(url: &str, id: Option<u32>) -> AnimeInfo {
        AnimeInfo {
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

        let client = Arc::new(Client::with_proxy(self.proxy).await?);

        let func = match self.site {
            Site::AW => Self::animeworld,
        };

        let sc = ScraperCollector::mutex();
        let tasks = query
            .iter()
            .map(|q| func(q, client.clone(), sc.clone()))
            .map(|f| async move { ok!(f.await) })
            .collect::<Vec<_>>();

        join_all(tasks).await;

        let res = sc.lock().await.clone();

        Ok(res)
    }

    async fn animeworld(
        query: &str,
        client: Arc<Client>,
        buf: Arc<Mutex<ScraperCollector>>,
    ) -> Result<()> {
        let search_url = format!("https://www.animeworld.tv/search?keyword={query}");

        let page = Self::parse(search_url, &client).await?;
        let results = {
            let div = Selector::parse("div.film-list").unwrap();
            let a = Selector::parse("a.name").unwrap();

            let elem = page
                .select(&div)
                .next()
                .ok_or_else(|| Error::with_msg("Request blocked, retry"))?;
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

        let choices = tui::get_choice(results, Some(query.replace("+", " ")))?;

        let pages = choices
            .iter()
            .map(|c| Self::parse(format!("https://www.animeworld.tv{c}"), &client))
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

                Self::item(&url.unwrap_or_default(), id)
            })
            .filter(|i| !i.url.is_empty())
            .collect::<Vec<_>>();

        if res.is_empty() {
            bail!(Error::UrlNotFound)
        }

        let mut buf = buf.lock().await;
        buf.extend(res);

        if buf.referrer.is_empty() {
            buf.referrer = "https://www.animeworld.tv/".to_string();
        }

        Ok(())
    }

    async fn parse(url: String, client: &Client) -> Result<Html> {
        let response = client.get(&url).send().await?.error_for_status()?;

        Ok(Html::parse_fragment(&response.text().await?))
    }
}

#[derive(Default, Debug)]
pub struct Client(RClient);

impl Client {
    async fn with_proxy(enable_proxy: bool) -> Result<Self> {
        let mut ctest = ClientBuilder::aw_ping().await.unwrap_or_default();
        ctest.push_str(ClientBuilder::COOKIE);

        let proxy = if enable_proxy {
            let res = reqwest::get(ClientBuilder::PROXY).await?.text().await?;
            res.split_ascii_whitespace()
                .choose(&mut rand::thread_rng())
                .map(|s| format!("https://{s}"))
        } else {
            None
        };

        ClientBuilder::default().ctest(&ctest).proxy(proxy).build()
    }

    #[cfg(test)]
    fn _builder() -> ClientBuilder {
        ClientBuilder::default()
    }
}

impl Deref for Client {
    type Target = RClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default, Debug)]
pub struct ClientBuilder {
    proxy: Option<String>,
    ctest: String,
}

#[rustfmt::skip]
impl<'a> ClientBuilder {
    const ACCEPT: &'a str = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8";
    const COOKIE: &'a str = "__ddg1=sti44Eo5SrS4IAwJPVFu; __cfduid=d1343ee68e09afafe0a4855d5c35e713f1619342282; _csrf=wSnjNmhifYyOPULeghB6Dloy;";
    const PROXY: &'a str = "https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=2000&country=all&ssl=all&anonymity=elite";
    const USER_AGENT: &'a str = "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";

    pub fn proxy(mut self, proxy: Option<String>) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn ctest(mut self, ctest: &str) -> Self {
        self.ctest = ctest.to_owned();
        self
    }

    pub fn build(self) -> Result<Client> {
        let mut headers = header::HeaderMap::new();
        
        let cookie = HeaderValue::from_str(&self.ctest).unwrap();
        headers.insert(header::COOKIE, cookie);
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));
        headers.insert(header::USER_AGENT, HeaderValue::from_static(Self::USER_AGENT));

        let mut builder = RClient::builder().default_headers(headers);

        if let Some(proxy) = self.proxy {
            let req_proxy = reqwest::Proxy::http(proxy).map_err(|_| Error::Proxy)?;
            builder = builder.proxy(req_proxy);
        }

        let client = builder.build()?;

        Ok(Client(client))
    }

    async fn aw_ping() -> Result<String> {
        let text = reqwest::get("https://www.animeworld.tv/").await?.text().await?;
        let res = Info::parse_aw_cookie(&text)?;

        Ok(res)
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
        let opt_proxy = Some("127.0.0.1".to_string());

        Client::_builder().proxy(opt_proxy).build().unwrap();
        Client::_builder().proxy(None).build().unwrap();

        Client::with_proxy(true).await.unwrap();
        Client::with_proxy(false).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_animeworld() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = ScraperCollector::mutex();
        let client = Arc::new(Client::_builder().build().unwrap());

        Scraper::animeworld("bunny girl", client, anime.clone())
            .await
            .unwrap();

        let anime = anime.lock().await.clone();
        let info = get_url(&anime.first().unwrap().url);

        assert_eq!(file, info)
    }

    #[tokio::test]
    #[ignore]
    async fn test_scraper() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Scraper::new("bunny girl").run().await.unwrap();

        let info = get_url(&anime.first().unwrap().url);

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

        anime.sort();
        files.sort_unstable();

        assert_eq!(anime, files)
    }
}
