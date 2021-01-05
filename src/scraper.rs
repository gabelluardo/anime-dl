pub use anyhow::{bail, Context, Result};

use crate::cli::Site;
use crate::utils::tui;

#[cfg(feature = "aes")]
use crate::utils::crypt;

use rand::seq::IteratorRandom;
use reqwest::{header, header::HeaderValue, Client, Url};
use scraper::{Html, Selector};

use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct ScraperItemDetails {
    pub url: String,
    pub id: Option<u32>,
}

#[derive(Debug, Default)]
pub struct ScraperItems(Vec<ScraperItemDetails>);

impl ScraperItems {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(url: String, id: Option<u32>) -> ScraperItemDetails {
        ScraperItemDetails { url, id }
    }
}

impl Deref for ScraperItems {
    type Target = Vec<ScraperItemDetails>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ScraperItems {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<ScraperItemDetails> for ScraperItems {
    fn from_iter<I: IntoIterator<Item = ScraperItemDetails>>(iter: I) -> Self {
        let mut c = ScraperItems::new();
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

    pub fn proxy(self, proxy: bool) -> Self {
        Self { proxy, ..self }
    }

    pub fn query(self, query: &'a str) -> Self {
        Self { query, ..self }
    }

    pub fn site(self, site: Option<Site>) -> Self {
        Self { site, ..self }
    }

    pub async fn run(&self) -> Result<ScraperItems> {
        // Concat strings if is passed with "" in shell
        let query = self.query.replace(" ", "+");

        match self.site {
            Some(Site::AW) | None => Self::animeworld(&query, self.proxy).await,
            Some(Site::AS) => bail!("Scraper `AS` parameter is deprecated"),
        }
    }

    async fn animeworld(query: &str, proxy: bool) -> Result<ScraperItems> {
        let client = ScraperClient::new(("AWCookietest", "https://animeworld.tv"), proxy).await?;

        let source = "https://www.animeworld.tv/search?keyword=";
        let search_url = format!("{}{}", source, query);

        let fragment = Self::parse(&search_url, &client).await?;
        let results = {
            let div = Selector::parse("div.film-list").unwrap();
            let a = Selector::parse("a.name").unwrap();

            match fragment.select(&div).next() {
                None => bail!("Request blocked, retry"),
                Some(e) => e
                    .select(&a)
                    .into_iter()
                    .map(|a| {
                        tui::Choice::from(
                            a.value().attr("href").expect("No link found").to_string(),
                            a.first_child()
                                .and_then(|a| a.value().as_text())
                                .expect("No name found")
                                .to_string(),
                        )
                    })
                    .collect::<Vec<_>>(),
            }
        };

        let choices = tui::get_choice(results)?;

        let mut anime = ScraperItems::new();
        for c in choices {
            let choice = format!("https://www.animeworld.tv{}", c);

            let fragment = Self::parse(&choice, &client).await?;
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

            anime.push(ScraperItems::item(url, id));
        }

        if anime.is_empty() {
            bail!("No anime found")
        }

        Ok(anime)
    }

    #[cfg(feature = "aes")]
    async fn parse(url: &str, client: &Client) -> Result<Html> {
        delay_for!(crypt::rand_range(100, 300));

        let response = client
            .get(url)
            .send()
            .await?
            .error_for_status()
            .context(format!("Unable to get anime page"))?;

        Ok(Html::parse_fragment(&response.text().await?))
    }

    #[cfg(not(feature = "aes"))]
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

type CookieInfo<'a> = (&'a str, &'a str);

struct ScraperClient(Client);

#[rustfmt::skip]
impl<'a> ScraperClient {
    const ACCEPT: &'a str = "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8";
    const COOKIES: &'a str = "__cfduid=df375aea9c761e29fe312136a2b0af16b1599087133;_csrf=ITVgw-fJSainaeRefw2IFwWG";
    const USER_AGENT: &'a str = "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-GB; rv:1.8.1.6) Gecko/20070725 Firefox/2.0.0.6";

    async fn new(site_props: CookieInfo<'_>, proxy: bool) -> Result<Self> {
        let mut client = Client::builder()
            .referer(true)
            .user_agent(Self::USER_AGENT)
            .default_headers(Self::set_headers(site_props).await?);

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

    async fn set_headers(site_props: CookieInfo<'_>) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        let cookies = Self::set_cookies(site_props).await?;

        headers.insert(header::COOKIE, HeaderValue::from_str(&cookies)?);
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));

        Ok(headers)
    }

    #[cfg(feature = "aes")]
    async fn set_cookies((cookie_name, url): CookieInfo<'_>) -> Result<String> {
        let response = reqwest::get(url).await?.text().await?;

        Ok(match crypt::extract_hex(&response, r"\(.(\d|\w)+.\)") {
            Ok(v) => {
                let (a, b, c) = (&v[0], &v[1], &v[2]);
                let output = crypt::encode(a, b, c)?;

                format!("{}={};", cookie_name, output)
            }
            Err(_) => String::from(Self::COOKIES),
        })
    }

    #[cfg(not(feature = "aes"))]
    async fn set_cookies(_: CookieInfo<'a>) -> Result<String> {
        Ok(String::from(Self::COOKIES))
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

    #[tokio::test]
    async fn test_client() {
        let proxy_client =
            ScraperClient::new(("AWCookietest", "https://animeworld.tv"), false).await;
        let no_proxy_client =
            ScraperClient::new(("ASCookie", "https://animesaturn.com"), true).await;

        proxy_client.unwrap();
        no_proxy_client.unwrap();
    }

    #[tokio::test]
    async fn test_animeworld() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Scraper::animeworld("bunny girl", false).await.unwrap();
        let info = Url::parse(&anime.first().unwrap().url)
            .unwrap()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .to_owned();

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
        let info = Url::parse(&anime.last().unwrap().url)
            .unwrap()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .to_owned();

        assert_eq!(file, info)
    }
}
