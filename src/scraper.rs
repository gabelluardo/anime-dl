use crate::cli::Site;
use crate::utils::{self, tui};

#[cfg(feature = "aes")]
use crate::utils::crypt;

use anyhow::{bail, Context, Result};
use reqwest::{header, header::HeaderValue, Client, Url};
use scraper::{Html, Selector};

use std::iter::FromIterator;
use std::ops::Deref;

#[derive(Clone, Debug)]
pub struct ScraperItemDetails {
    pub url: String,
    pub id: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct ScraperItems {
    items: Vec<ScraperItemDetails>,
}

impl ScraperItems {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(url: String, id: Option<u32>) -> ScraperItemDetails {
        ScraperItemDetails { url, id }
    }

    pub fn push(&mut self, item: ScraperItemDetails) {
        self.items.push(item)
    }

    pub fn to_vec(&self) -> Vec<ScraperItemDetails> {
        self.items.clone()
    }

    pub fn first(&self) -> Option<&ScraperItemDetails> {
        self.items.first()
    }

    pub fn last(&self) -> Option<&ScraperItemDetails> {
        self.items.last()
    }

    pub fn iter(&self) -> ScraperItemsIterator {
        self.into_iter()
    }
}

impl FromIterator<ScraperItemDetails> for ScraperItems {
    fn from_iter<I: IntoIterator<Item = ScraperItemDetails>>(iter: I) -> Self {
        let mut c = ScraperItems::new();
        c.extend(iter);
        c
    }
}

impl Extend<ScraperItemDetails> for ScraperItems {
    fn extend<T: IntoIterator<Item = ScraperItemDetails>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |c| self.push(c))
    }
}

pub struct ScraperItemsIntoIterator {
    iter: ::std::vec::IntoIter<ScraperItemDetails>,
}

impl<'a> IntoIterator for ScraperItems {
    type Item = ScraperItemDetails;
    type IntoIter = ScraperItemsIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        ScraperItemsIntoIterator {
            iter: self.items.into_iter(),
        }
    }
}

impl<'a> Iterator for ScraperItemsIntoIterator {
    type Item = ScraperItemDetails;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

pub struct ScraperItemsIterator<'a> {
    iter: ::std::slice::Iter<'a, ScraperItemDetails>,
}

impl<'a> IntoIterator for &'a ScraperItems {
    type Item = &'a ScraperItemDetails;
    type IntoIter = ScraperItemsIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ScraperItemsIterator {
            iter: self.items.iter(),
        }
    }
}

impl<'a> Iterator for ScraperItemsIterator<'a> {
    type Item = &'a ScraperItemDetails;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Default)]
pub struct Scraper {
    proxy: bool,
    query: String,
    site: Option<Site>,
}

impl<'a> Scraper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn proxy(self, proxy: bool) -> Self {
        Self { proxy, ..self }
    }

    pub fn query(self, query: &str) -> Self {
        Self {
            query: query.to_owned(),
            ..self
        }
    }

    pub fn site(self, site: Option<Site>) -> Self {
        Self {
            site: site.to_owned(),
            ..self
        }
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
                None => bail!("Request blocked, retry"),
            }
        };

        let choices = tui::get_choice(results)?;

        let mut anime = ScraperItems::new();
        for choice in choices {
            let choice = format!("https://www.animeworld.tv{}", choice);

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

        Ok(anime)
    }

    #[cfg(feature = "aes")]
    async fn parse(url: &str, client: &Client) -> Result<Html> {
        delay_for!(utils::rand_range(100, 300));

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
            .context(format!("Unable to get anime page"))?;

        Ok(Html::parse_fragment(&response.text().await?))
    }
}

type CookieInfo<'a> = (&'a str, &'a str);

struct ScraperClient(Client);
impl<'a> ScraperClient {
    const USER_AGENT: &'a str = "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-GB; rv:1.8.1.6)\
 Gecko/20070725 Firefox/2.0.0.6";
    const ACCEPT: &'a str = "text/html,application/xhtml+xml,application/\
    xml;q=0.9,image/webp,*/*;q=0.8";
    const COOKIES: &'a str = "__cfduid=df375aea9c761e29fe312136a2b0af16b1599087133;\
    _csrf=ITVgw-fJSainaeRefw2IFwWG";

    async fn new(site_props: CookieInfo<'a>, enable_proxy: bool) -> Result<Self> {
        let headers = Self::set_headers(site_props).await?;
        let client = Client::builder()
            .referer(true)
            .user_agent(Self::USER_AGENT)
            .default_headers(headers);

        let client = if enable_proxy {
            client.proxy(Self::set_proxy().await?)
        } else {
            client
        };

        Ok(Self(client.build().unwrap()))
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

        let proxies = response
            .split_ascii_whitespace()
            .into_iter()
            .collect::<Vec<_>>();

        reqwest::Proxy::http(&format!(
            "http://{}",
            proxies[utils::rand_range(0, proxies.len())]
        ))
        .context("Unable to parse proxyscrape")
    }

    async fn set_headers(site_props: CookieInfo<'a>) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        let cookies = Self::set_cookies(site_props).await?;

        headers.insert(header::COOKIE, HeaderValue::from_str(&cookies)?);
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));

        Ok(headers)
    }

    #[cfg(feature = "aes")]
    async fn set_cookies((cookie_name, url): CookieInfo<'a>) -> Result<String> {
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
        let anime = Scraper::animeworld("bunny girl", false).await.unwrap();
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
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
        let s = Scraper::new();
        let anime = s
            .site(Some(Site::AW))
            .query("bunny girl")
            .run()
            .await
            .unwrap();
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let info = Url::parse(&anime.last().unwrap().url)
            .unwrap()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .to_owned();

        assert_eq!(file, info)
    }
}
