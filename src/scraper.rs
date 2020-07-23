use crate::cli::Site;
use crate::utils::*;

use anyhow::{bail, Context, Result};
use rand::prelude::*;
use reqwest::{header, header::HeaderValue, Client};
use scraper::{Html, Selector};

// AWCookietest for AW and ASCookie for AS
const COOKIE: &str = "__cfduid=d6217e694ae44946bd69c717bbb7577361595537028;\
    _csrf=SqYj4gMXcEPlL9DROQKIYcSk;AWCookietest=f731d67b0c1777ceadb3898e7e4aca8c;\
    ASCookie=e40207ab9dd09e111a5e154cd25da264;expandedPlayer=false";
const USER_AGENT: &str = "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-GB; rv:1.8.1.6)\
 Gecko/20070725 Firefox/2.0.0.6";
const ACCEPT: &str = "text/html,application/xhtml+xml,application/\
    xml;q=0.9,image/webp,*/*;q=0.8";

pub struct Scraper {
    site: Site,
    query: String,
}

impl Scraper {
    pub fn new(site: Site, query: String) -> Self {
        Self { site, query }
    }

    pub async fn run(&self) -> Result<Vec<String>> {
        // Concat string if is passed with "" in shell
        let query = self.query.replace(" ", "+");

        match self.site {
            Site::AW => Self::animeworld(&query).await,
            Site::AS => Self::animesaturn(&query).await,
        }
    }

    async fn init_client() -> Result<Client> {
        let proxy = {
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

            format!(
                "http://{}",
                proxies[thread_rng().gen_range(0, proxies.len())]
            )
        };

        let mut headers = header::HeaderMap::new();

        headers.insert(header::COOKIE, HeaderValue::from_static(COOKIE));
        headers.insert(header::ACCEPT, HeaderValue::from_static(ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));

        Ok(Client::builder()
            .referer(true)
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .proxy(reqwest::Proxy::http(&proxy)?)
            .build()
            .unwrap())
    }

    async fn animeworld(query: &str) -> Result<Vec<String>> {
        let client = Self::init_client().await?;

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
                        (
                            a.value().attr("href").expect("No link found"),
                            a.first_child()
                                .and_then(|a| a.value().as_text())
                                .expect("No name found") as &str,
                        )
                    })
                    .collect::<Vec<_>>(),
                _ => bail!("Request blocked, retry"),
            }
        };

        let choices = prompt_choices(results)?;

        let mut urls = vec![];
        for choice in choices {
            let choice = format!("https://www.animeworld.tv{}", choice);

            let fragment = Self::parse(&choice, &client).await?;
            let results = {
                let a = Selector::parse(r#"a[id="alternativeDownloadLink"]"#).unwrap();

                fragment
                    .select(&a)
                    .last()
                    .and_then(|a| a.value().attr("href"))
            };

            let url = match results {
                Some(u) => u.to_string(),
                _ => bail!("No link found"),
            };
            urls.push(url);
        }

        Ok(urls)
    }

    async fn animesaturn(query: &str) -> Result<Vec<String>> {
        let client = Self::init_client().await?;

        let source = "https://www.animesaturn.com/animelist?search=";
        let search_url = format!("{}{}", source, query);

        let fragment = Self::parse(&search_url, &client).await?;
        let results = {
            let div = Selector::parse("div.info-archivio").unwrap();
            let a = Selector::parse("a.badge-archivio").unwrap();

            match fragment.select(&div).next() {
                Some(_) => fragment
                    .select(&a)
                    .into_iter()
                    .map(|a| {
                        (
                            a.value().attr("href").expect("No link found"),
                            a.first_child()
                                .and_then(|a| a.value().as_text())
                                .expect("No name found") as &str,
                        )
                    })
                    .collect::<Vec<_>>(),
                _ => bail!("Request blocked, retry"),
            }
        };

        let choices = prompt_choices(results)?;

        let mut urls = vec![];
        for choice in choices {
            let fragment = Self::parse(&choice, &client).await?;
            let results = {
                let a = Selector::parse("a.bottone-ep").unwrap();

                fragment
                    .select(&a)
                    .next()
                    .and_then(|a| a.value().attr("href"))
                    .expect("No link found")
            };

            let fragment = Self::parse(&results, &client).await?;
            let results = {
                let div = Selector::parse("div.card-body").unwrap();
                let a = Selector::parse("a").unwrap();

                fragment
                    .select(&div)
                    .next()
                    .and_then(|div| div.select(&a).next())
                    .and_then(|a| a.value().attr("href"))
                    .expect("No link found")
            };

            let fragment = Self::parse(&results, &client).await?;
            let results = {
                let source = Selector::parse(r#"source[type="video/mp4"]"#).unwrap();

                fragment
                    .select(&source)
                    .next()
                    .and_then(|s| s.value().attr("src"))
            };

            // delay_for!(300);
            let url = match results {
                Some(u) => match client.get(u).send().await?.error_for_status() {
                    Ok(_) => u.to_string(),
                    _ => Self::as_change_server(&fragment, &client).await?,
                },
                _ => Self::as_change_server(&fragment, &client).await?,
            };
            urls.push(url);
        }

        Ok(urls)
    }

    async fn as_change_server(fragment: &Html, client: &Client) -> Result<String> {
        let results = {
            let div = Selector::parse("div.button").unwrap();
            let a = Selector::parse("a").unwrap();
            let opt = fragment
                .select(&div)
                .next()
                .and_then(|div| div.select(&a).last())
                .and_then(|a| a.value().attr("href"));

            match opt {
                Some(v) => v,
                _ => bail!("No link found"),
            }
        };
        let fragment = Self::parse(results, client).await?;

        let url = {
            let source = Selector::parse(r#"source[type="video/mp4"]"#).unwrap();
            let opt = fragment
                .select(&source)
                .next()
                .and_then(|s| s.value().attr("src"));

            match opt {
                Some(v) => v.to_string(),
                _ => bail!("No link found"),
            }
        };

        Ok(url)
    }

    async fn parse(url: &str, client: &Client) -> Result<Html> {
        delay_for!(thread_rng().gen_range(100, 400));

        let response = client
            .get(url)
            .send()
            .await?
            .error_for_status()
            .context(format!("Unable to get anime page"))?;

        Ok(Html::parse_fragment(&response.text().await?))
    }
}
