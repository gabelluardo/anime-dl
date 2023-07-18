use std::sync::Arc;

use anyhow::{bail, Context, Result};
use futures::future::join_all;
use reqwest::Url;
use scraper::Selector;
use tokio::sync::Mutex;

use crate::anime::AnimeInfo;
use crate::errors::RemoteError;
use crate::scraper::Client;
use crate::tui;

#[async_trait::async_trait]
pub trait Archive {
    fn referrer() -> Option<String>;
    async fn run(param: (&str, Arc<Client>, Arc<Mutex<Vec<AnimeInfo>>>)) -> Result<()>;
}

pub struct AnimeWorld;
#[async_trait::async_trait]
impl Archive for AnimeWorld {
    fn referrer() -> Option<String> {
        Some(String::from("https://www.animeworld.tv"))
    }

    async fn run(param: (&str, Arc<Client>, Arc<Mutex<Vec<AnimeInfo>>>)) -> Result<()> {
        async fn inner(client: Arc<Client>, page_url: String) -> Result<AnimeInfo> {
            let page = client.parse_url(&page_url).await?;
            let a = Selector::parse(r#"a[id="alternativeDownloadLink"]"#).unwrap();
            let mut url = page.select(&a).last().and_then(|a| a.value().attr("href"));
            if url.is_none() || url == Some("") {
                let a = Selector::parse(r#"a[id="downloadLink"]"#).unwrap();
                url = page.select(&a).last().and_then(|a| a.value().attr("href"))
            }
            if url.is_none() || url == Some("") {
                let a = Selector::parse(r#"a[id="customDownloadButton"]"#).unwrap();
                url = page.select(&a).last().and_then(|a| a.value().attr("href"))
            }
            let url = match url {
                Some(u) => u.replace("download-file.php?id=", ""),
                _ => bail!(RemoteError::UrlNotFound),
            };

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

            Ok(AnimeInfo::new(&url, id))
        }

        let (query, client, vec) = param;
        let search_results = {
            let referrer = Self::referrer().unwrap();
            let search_url = format!("{referrer}/search?keyword={query}");
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

        let mut tasks = vec![];
        for c in selected.iter() {
            tasks.push(inner(client.clone(), Self::referrer().unwrap() + c))
        }
        let res = join_all(tasks)
            .await
            .into_iter()
            .filter_map(|a| a.ok())
            .collect::<Vec<_>>();
        if res.is_empty() {
            bail!(RemoteError::UrlNotFound)
        }

        let mut lock = vec.lock().await;
        lock.extend(res);

        Ok(())
    }
}

pub struct Placeholder;
#[async_trait::async_trait]
impl Archive for Placeholder {
    fn referrer() -> Option<String> {
        unimplemented!()
    }

    async fn run(_param: (&str, Arc<Client>, Arc<Mutex<Vec<AnimeInfo>>>)) -> Result<()> {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
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
    #[ignore]
    async fn test_remote_animeworld() {
        let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
        let anime = Arc::new(Mutex::new(Vec::new()));
        let client = Arc::new(Client::default());

        let param = ("bunny girl", client, anime.clone());
        AnimeWorld::run(param).await.unwrap();

        let anime = anime.lock().await.clone();
        let info = get_url(&anime.first().unwrap().origin);

        assert_eq!(file, info)
    }
}
