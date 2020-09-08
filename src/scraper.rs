use crate::cli::Site;
use crate::utils::{self, crypt, tui};

use anyhow::{bail, Context, Result};
use reqwest::{header, header::HeaderValue, Client, Url};
use scraper::{Html, Selector};

use std::iter::FromIterator;

#[derive(Clone)]
pub struct ScraperItemDetails {
    pub url: String,
    pub id: Option<u32>,
}

#[derive(Default, Clone)]
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
        iter.into_iter().for_each(drop)
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
    site: Option<Site>,
    query: String,
}

impl<'a> Scraper {
    const USER_AGENT: &'a str = "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-GB; rv:1.8.1.6)\
 Gecko/20070725 Firefox/2.0.0.6";
    const ACCEPT: &'a str = "text/html,application/xhtml+xml,application/\
    xml;q=0.9,image/webp,*/*;q=0.8";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn site(self, site: &Site) -> Self {
        Self {
            site: Some(site.to_owned()),
            ..self
        }
    }

    pub fn query(self, query: &str) -> Self {
        Self {
            query: query.to_owned(),
            ..self
        }
    }

    pub async fn run(&self) -> Result<ScraperItems> {
        // Concat string if is passed with "" in shell
        let query = self.query.replace(" ", "+");

        match self.site {
            Some(Site::AW) => Self::animeworld(&query).await,
            Some(Site::AS) => bail!("Scraper `AS` parameter is deprecated"),
            None => bail!("Missing Scraper `site` parameter"),
        }
    }

    async fn init_client(_site: Option<(&str, &str)>) -> Result<Client> {
        let mut headers = header::HeaderMap::new();

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

            format!("http://{}", proxies[utils::rand_range(0, proxies.len())])
        };

        let mut cookies = match _site {
            Some((cookie_name, url)) => {
                let response = reqwest::get(url).await?.text().await?;

                match crypt::extract_hex(&response, r"\(.(\d|\w)+.\)") {
                    Ok(v) => {
                        let (a, b, c) = (&v[0], &v[1], &v[2]);
                        let output = crypt::encode(a, b, c)?;

                        format!("{}={};", cookie_name, output)
                    }
                    _ => String::new(),
                }
            }
            _ => String::new(),
        };

        cookies.push_str(
            "__cfduid=df375aea9c761e29fe312136a2b0af16b1599087133;\
            _csrf=ITVgw-fJSainaeRefw2IFwWG",
        );

        headers.insert(header::COOKIE, HeaderValue::from_str(&cookies)?);
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("it"));

        Ok(Client::builder()
            .referer(true)
            .user_agent(Self::USER_AGENT)
            .default_headers(headers)
            .proxy(reqwest::Proxy::http(&proxy)?)
            .build()
            .unwrap())
    }

    async fn animeworld(query: &str) -> Result<ScraperItems> {
        // if doesn't work add: `Some(("AWCookietest", "https://animeworld.tv"))`
        let client = Self::init_client(None).await?;

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
                _ => bail!("Request blocked, retry"),
            }
        };

        let choices = tui::prompt_choices(results)?;

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
                _ => bail!("No link found"),
            };

            anime.push(ScraperItems::item(url, id));
        }

        Ok(anime)
    }

    async fn parse(url: &str, client: &Client) -> Result<Html> {
        // NOTE: Uncomment if is implemented an antiscraper
        // delay_for!(rand_range(100, 300));

        let response = client
            .get(url)
            .send()
            .await?
            .error_for_status()
            .context(format!("Unable to get anime page"))?;

        Ok(Html::parse_fragment(&response.text().await?))
    }

    // DEPRECATED: since 1.0.4
    #[allow(dead_code)]
    async fn animesaturn(query: &str) -> Result<Vec<String>> {
        // if doesn't work add: `Some(("ASCookie", "https://animesaturn.com"))`
        let client = Self::init_client(None).await?;

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
                        tui::Choice::from(
                            a.value().attr("href").expect("No link found").to_string(),
                            a.first_child()
                                .and_then(|a| a.value().as_text())
                                .expect("No name found")
                                .to_string(),
                        )
                    })
                    .collect::<Vec<_>>(),
                _ => bail!("Request blocked, retry"),
            }
        };

        let choices = tui::prompt_choices(results)?;

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

    // DEPRECATED: since 1.0.4
    #[allow(dead_code)]
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Url;

    #[tokio::test]
    async fn test_init_client() {
        let aw_c = Scraper::init_client(Some(("AWCookietest", "https://animeworld.tv"))).await;
        let as_c = Scraper::init_client(Some(("ASCookie", "https://animesaturn.com"))).await;
        let n_c = Scraper::init_client(None).await;

        aw_c.unwrap();
        as_c.unwrap();
        n_c.unwrap();
    }

    #[tokio::test]
    async fn test_animeworld() {
        let anime = Scraper::animeworld("bunny girl").await.unwrap();
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let info = Url::parse(&anime.first().unwrap().url)
            .unwrap()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .to_owned();

        assert_eq!(file, info)
    }

    // #[tokio::test]
    // async fn test_animesaturn() {
    //     let url = Scraper::animesaturn("bunny girl").await.unwrap();
    //     let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
    //     let info = Url::parse(url.first().unwrap())
    //         .unwrap()
    //         .path_segments()
    //         .and_then(|segments| segments.last())
    //         .unwrap()
    //         .to_owned();

    //     assert_eq!(file, info)
    // }

    #[tokio::test]
    async fn test_scraper() {
        let s = Scraper::new();
        let anime = s.site(&Site::AW).query("bunny girl").run().await.unwrap();
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let info = Url::parse(&anime.last().unwrap().url)
            .unwrap()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap()
            .to_owned();

        assert_eq!(file, info)
    }

    #[tokio::test]
    #[should_panic(expected = "Missing Scraper `site` parameter")]
    async fn test_scraper_err() {
        Scraper::new().run().await.unwrap();
    }
}
