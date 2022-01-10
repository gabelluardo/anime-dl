use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use futures::future::join_all;
use rand::seq::IteratorRandom;
use reqwest::{header, header::HeaderValue, Client as RClient, Url};
use scraper::{Html, Selector};
use tokio::sync::Mutex;

use crate::cli::Site;
use crate::errors::{Error, Result};
use crate::utils::{tui, Info};

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

        let client = Arc::new(Client::with_proxy(self.proxy).await?);

        let func = match self.site {
            Some(Site::AW) | None => Self::animeworld,
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
        let search_url = format!("https://www.animeworld.tv/search?keyword={}", query);

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

        Ok(Html::parse_fragment(&response.text().await?))
    }
}

#[derive(Default, Debug)]
pub struct Client(RClient);

impl Client {
    async fn _new() -> Result<Self> {
        ClientBuilder::default().build().await
    }

    fn _builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    async fn with_proxy(p: bool) -> Result<Self> {
        ClientBuilder::default().proxy(p).build().await
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
    proxy: bool,
}

#[rustfmt::skip]
impl<'a> ClientBuilder {
    const ACCEPT: &'a str = "text/html,application/xhtml+xml,application/xml; q=0.9,image/webp,*/*; q=0.8";
    const COOKIE: &'a str = "__ddg1=sti44Eo5SrS4IAwJPVFu; __cfduid=d1343ee68e09afafe0a4855d5c35e713f1619342282; _csrf=wSnjNmhifYyOPULeghB6Dloy;";
    const PROXY: &'a str = "https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=2000&country=all&ssl=all&anonymity=elite";
    const USER_AGENT: &'a str = "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-GB; rv:1.8.1.6) Gecko/20070725 Firefox/2.0.0.6";

    pub fn proxy(mut self, p: bool) -> Self {
        self.proxy = p;
        self
    }

    pub async fn build(self) -> Result<Client> {
        let mut ctest = self.aw_ping().await.unwrap_or_default();
        ctest.push_str(Self::COOKIE);

        let cookie = HeaderValue::from_str(&ctest).unwrap();
        let mut headers = header::HeaderMap::new();
        headers.insert(header::COOKIE, cookie);
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));
        headers.insert(header::USER_AGENT, HeaderValue::from_static(Self::USER_AGENT));

        let mut builder = RClient::builder().default_headers(headers);

        if self.proxy {
            let res = reqwest::get(Self::PROXY).await?.text().await?;
            let proxy = res
                .split_ascii_whitespace()
                .choose(&mut rand::thread_rng())
                .map(|s| format!("https://{}", s))
                .unwrap_or_default();

            let p = reqwest::Proxy::http(proxy).map_err(|_| Error::Proxy)?;

            builder = builder.proxy(p);
        }

        let client = builder.build()?;

        Ok(Client(client))
    }

    async fn aw_ping(&self) -> Result<String> {
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
        Client::_builder().proxy(true).build().await.unwrap();
        Client::_builder().proxy(false).build().await.unwrap();

        Client::with_proxy(true).await.unwrap();
        Client::with_proxy(false).await.unwrap();
    }

    #[tokio::test]
    async fn test_animeworld() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = ScraperCollector::mutex();
        let client = Arc::new(Client::_new().await.unwrap());

        Scraper::animeworld("bunny girl", client, anime.clone())
            .await
            .unwrap();
        let anime = anime.lock().await.clone();

        let info = get_url(&anime.first().unwrap().url);

        assert_eq!(file, info)
    }

    #[tokio::test]
    async fn test_scraper() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Scraper::new(false, "bunny girl", Some(Site::AW))
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

        let anime = Scraper::new(
            false,
            "bunny girl, tsuredure children, promare",
            Some(Site::AW),
        )
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
