pub use anyhow::{bail, Context, Result};

use crate::cli::Site;
use crate::utils::tui;

use rand::seq::IteratorRandom;
use reqwest::{header, header::HeaderValue, Client, Url};
use scraper::{Html, Selector};

use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct ScraperItem {
    pub id: Option<u32>,
    pub url: String,
}

#[derive(Debug, Default)]
pub struct ScraperCollector {
    pub items: Vec<ScraperItem>,
    pub referer: String,
}

impl ScraperCollector {
    pub fn new() -> Self {
        Self::default()
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
pub struct Scraper<'a> {
    proxy: bool,
    query: &'a str,
    site: Option<Site>,
}

impl<'a> Scraper<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn query(mut self, query: &'a str) -> Self {
        self.query = query;
        self
    }

    pub fn site(mut self, site: Option<Site>) -> Self {
        self.site = site;
        self
    }

    pub fn collector() -> ScraperCollector {
        ScraperCollector::new()
    }

    pub fn item(url: String, id: Option<u32>) -> ScraperItem {
        ScraperItem { id, url }
    }

    pub async fn run(&self) -> Result<ScraperCollector> {
        let query = self
            .query
            .split(",")
            .map(|s| s.trim().replace(" ", "+"))
            .collect::<Vec<_>>();

        let mut res = Self::collector();
        for q in &query {
            match self.site {
                Some(Site::AW) | None => Self::animeworld(q, self.proxy, &mut res).await?,
                Some(Site::AS) => bail!("Scraper `AS` parameter is deprecated"),
            }
        }

        if res.is_empty() {
            bail!("No anime found")
        }

        Ok(res)
    }

    async fn animeworld(query: &str, proxy: bool, buf: &mut ScraperCollector) -> Result<()> {
        let client = ScraperClient::new(proxy).await?;
        let search_url = format!("https://www.animeworld.tv/search?keyword={}", query);

        let mut fragment = Self::parse(&search_url, &client).await?;
        let results = {
            let div = Selector::parse("div.film-list").unwrap();
            let a = Selector::parse("a.name").unwrap();

            match fragment.select(&div).next() {
                Some(e) => e
                    .select(&a)
                    .into_iter()
                    .map(|a| {
                        tui::Choice::new(
                            a.value().attr("href").expect("No link found").to_string(),
                            a.first_child()
                                .and_then(|a| a.value().as_text())
                                .expect("No name found")
                                .to_string(),
                        )
                    })
                    .collect::<Vec<_>>(),
                None => bail!("Request blocked, retry"),
            }
        };

        let choices = tui::get_choice(results).await?;

        for c in choices {
            let choice = format!("https://www.animeworld.tv{}", c);

            fragment = Self::parse(&choice, &client).await?;
            let url = {
                let a = Selector::parse(r#"a[id="alternativeDownloadLink"]"#).unwrap();

                fragment
                    .select(&a)
                    .last()
                    .and_then(|a| a.value().attr("href"))
            };

            let id = {
                let a = Selector::parse(r#"a[id="anilist-button"]"#).unwrap();

                fragment
                    .select(&a)
                    .last()
                    .and_then(|a| a.value().attr("href"))
                    .map(|u| {
                        Url::parse(&u)
                            .unwrap()
                            .path_segments()
                            .and_then(|segments| segments.last().unwrap().parse::<u32>().ok())
                            .unwrap()
                    })
            };

            let url = match url {
                Some(u) => u.to_string(),
                None => bail!("No link found"),
            };

            buf.push(Self::item(url, id));
        }

        if buf.referer.is_empty() {
            buf.referer = "https://www.animeworld.tv/".to_string();
        }

        Ok(())
    }

    async fn parse(url: &str, client: &Client) -> Result<Html> {
        let response = client
            .get(url)
            .send()
            .await?
            .error_for_status()
            .context("Unable to get anime page")?;

        Ok(Html::parse_fragment(&response.text().await?))
    }
}

struct ScraperClient(Client);

impl<'a> ScraperClient {
    #[rustfmt::skip]
    const ACCEPT: &'a str = "text/html,application/xhtml+xml,application/xml; q=0.9,image/webp,*/*; q=0.8";
    const COOKIES: &'a str = "__cfduid=d03255bed084571c421edd313dbfd5fe31610142561; _csrf=PLwPaldqI-hCpuZzS8wfLnkP; expandedPlayer=false; theme=dark";
    const USER_AGENT: &'a str = "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-GB; rv:1.8.1.6) Gecko/20070725 Firefox/2.0.0.6";

    async fn new(proxy: bool) -> Result<Self> {
        let mut client = Client::builder()
            .user_agent(Self::USER_AGENT)
            .default_headers(Self::set_headers().await?);

        if proxy {
            client = client.proxy(Self::set_proxy().await?);
        }

        Ok(Self(client.build()?))
    }

    async fn set_proxy() -> Result<reqwest::Proxy> {
        let response = reqwest::get(
            "https://api.proxyscrape.com/\
                    ?request=getproxies&proxytype=http\
                    &timeout=2000&country=all&ssl=all&anonymity=elite",
        )
        .await?
        .text()
        .await?;

        let proxy = response
            .split_ascii_whitespace()
            .choose(&mut rand::thread_rng())
            .map(|s| format!("http://{}", s));

        reqwest::Proxy::http(&proxy.unwrap()).context("Unable to parse proxyscrape")
    }

    async fn set_headers() -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();

        headers.insert(header::COOKIE, HeaderValue::from_static(Self::COOKIES));
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));

        Ok(headers)
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
    use super::*;
    use reqwest::Url;

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
        let mut anime = ScraperCollector::new();

        Scraper::animeworld("bunny girl", false, &mut anime)
            .await
            .unwrap();

        let info = get_url(&anime.first().unwrap().url);

        assert_eq!(file, info)
    }

    #[tokio::test]
    async fn test_scraper() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Scraper::new()
            .site(Some(Site::AW))
            .query("bunny girl")
            .run()
            .await
            .unwrap();

        let info = get_url(&anime.first().unwrap().url);

        assert_eq!(file, info)
    }

    #[tokio::test]
    async fn test_scraper_multi() {
        let files = vec![
            "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4",
            "TsurezureChildren_Ep_01_SUB_ITA.mp4",
            "Promare_Movie_ITA.mp4",
        ];

        let anime = Scraper::new()
            .site(Some(Site::AW))
            .query("bunny girl, tsuredure children, promare")
            .run()
            .await
            .unwrap();

        let anime = anime
            .iter()
            .map(|a| {
                Url::parse(&a.url)
                    .unwrap()
                    .path_segments()
                    .and_then(|segments| segments.last())
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<String>>();

        assert_eq!(anime, files)
    }
}
