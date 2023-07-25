use std::sync::Arc;

use anyhow::{bail, Context, Result};

use reqwest::Url;
use scraper::{Html, Selector};
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
        let (query, client, vec) = param;
        let search_results = {
            let referrer = Self::referrer().unwrap();
            let search_url = format!("{referrer}/search?keyword={query}");
            let search_page = client.parse_url(&search_url).await?;
            let anime_list = Selector::parse("div.film-list").unwrap();
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

        let mut res: Vec<AnimeInfo> = vec![];
        for c in selected.iter() {
            let page = client.parse_url(&(Self::referrer().unwrap() + c)).await?;
            match Self::parser(page) {
                Ok(info) => res.push(info),
                _ => continue,
            }
        }

        if res.is_empty() {
            bail!(RemoteError::UrlNotFound)
        }

        let mut lock = vec.lock().await;
        lock.extend(res);

        Ok(())
    }
}

impl AnimeWorld {
    fn parser(page: Html) -> Result<AnimeInfo> {
        let url = Self::parse_url(&page)?;
        let id = Self::parse_id(&page);
        let episodes = Self::parse_episodes(&page);

        Ok(AnimeInfo::new(&url, id, episodes))
    }

    fn parse_url(page: &Html) -> Result<String> {
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

        Ok(url)
    }

    fn parse_id(page: &Html) -> Option<u32> {
        let btn = Selector::parse(r#"a[id="anilist-button"]"#).unwrap();
        page.select(&btn)
            .last()
            .and_then(|a| a.value().attr("href"))
            .and_then(|u| {
                Url::parse(u)
                    .unwrap()
                    .path_segments()
                    .and_then(|s| s.last())
                    .and_then(|s| s.parse::<u32>().ok())
            })
    }

    fn parse_episodes(page: &Html) -> Option<(u32, u32)> {
        let range = Selector::parse("div.range").unwrap();
        let span = Selector::parse("span.rangetitle").unwrap();
        match page.select(&range).next() {
            Some(range) if range.select(&span).next().is_some() => {
                let mut list = range.select(&span);

                let start = list
                    .next()?
                    .inner_html()
                    .split_ascii_whitespace()
                    .next()?
                    .parse::<u32>()
                    .ok()?;
                let end = list
                    .last()?
                    .inner_html()
                    .split_ascii_whitespace()
                    .last()?
                    .parse::<u32>()
                    .ok()?;

                Some((start, end))
            }
            _ => {
                let ul = Selector::parse("ul.episodes").unwrap();
                let a = Selector::parse("a").unwrap();
                let mut list = page.select(&ul).next().unwrap().select(&a);

                let start = list.next()?.inner_html().parse::<u32>().ok()?;
                let end = list.last()?.inner_html().parse::<u32>().ok()?;

                Some((start, end))
            }
        }
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

    #[test]
    fn test_pase_episodes_animeworld() {
        let html = r#"
            <ul class="episodes range acrive: data-range-id="0" style="display: block;">
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="1" data-num="1" data-base="1" data-comment="1" href="/play/anime_name/id">1</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="2" data-num="2" data-base="2" data-comment="2" href="/play/anime_name/id">2</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="3" data-num="3" data-base="3" data-comment="3" href="/play/anime_name/id">3</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="4" data-num="4" data-base="4" data-comment="4" href="/play/anime_name/id">4</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="5" data-num="5" data-base="5" data-comment="5" href="/play/anime_name/id">5</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="6" data-num="6" data-base="6" data-comment="6" href="/play/anime_name/id">6</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="7" data-num="7" data-base="7" data-comment="7" href="/play/anime_name/id">7</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="8" data-num="8" data-base="8" data-comment="8" href="/play/anime_name/id">8</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="9" data-num="9" data-base="9" data-comment="9" href="/play/anime_name/id">9</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="10" data-num="10" data-base="10" data-comment="10" href="/play/anime_name/id">10</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="11" data-num="11" data-base="11" data-comment="11" href="/play/anime_name/id">11</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="12" data-num="12" data-base="12" data-comment="12" href="/play/anime_name/id">12</a>
                </li>
            </ul>"#;
        let fragment = Html::parse_fragment(html);
        let episodes = AnimeWorld::parse_episodes(&fragment).unwrap();

        assert_eq!(episodes, (1, 12));

        let html = r#"
            <div class="range">
                <span data-range-id="0" class="rangetitle active">1 - 55</span>           
                <span data-range-id="1" class="rangetitle">56 - 111</span>           
                <span data-range-id="2" class="rangetitle">112 - 162</span>           
                <span data-range-id="3" class="rangetitle">163 - 212</span>           
                <span data-range-id="4" class="rangetitle">213 - 262</span>           
                <span data-range-id="5" class="rangetitle">263 - 312</span>           
                <span data-range-id="6" class="rangetitle">313 - 362</span>           
                <span data-range-id="7" class="rangetitle">363 - 412</span>           
                <span data-range-id="8" class="rangetitle">413 - 462</span>           
                <span data-range-id="9" class="rangetitle">463 - 500</span>           
            </div>"#;
        let fragment = Html::parse_fragment(html);
        let episodes = AnimeWorld::parse_episodes(&fragment).unwrap();

        assert_eq!(episodes, (1, 500));

        let html = r#"
            <div class="range"></div>
            <ul class="episodes range acrive: data-range-id="0" style="display: block;">
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="1" data-num="1" data-base="1" data-comment="1" href="/play/anime_name/id">1</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="2" data-num="2" data-base="2" data-comment="2" href="/play/anime_name/id">2</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="3" data-num="3" data-base="3" data-comment="3" href="/play/anime_name/id">3</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="4" data-num="4" data-base="4" data-comment="4" href="/play/anime_name/id">4</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="5" data-num="5" data-base="5" data-comment="5" href="/play/anime_name/id">5</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="6" data-num="6" data-base="6" data-comment="6" href="/play/anime_name/id">6</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="7" data-num="7" data-base="7" data-comment="7" href="/play/anime_name/id">7</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="8" data-num="8" data-base="8" data-comment="8" href="/play/anime_name/id">8</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="9" data-num="9" data-base="9" data-comment="9" href="/play/anime_name/id">9</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="10" data-num="10" data-base="10" data-comment="10" href="/play/anime_name/id">10</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="11" data-num="11" data-base="11" data-comment="11" href="/play/anime_name/id">11</a>
                </li>
                <li class="episode">
                    <a data-episode-id="id" data-id="id" data-episode-num="12" data-num="12" data-base="12" data-comment="12" href="/play/anime_name/id">12</a>
                </li>
            </ul>"#;
        let fragment = Html::parse_fragment(html);
        let episodes = AnimeWorld::parse_episodes(&fragment).unwrap();

        assert_eq!(episodes, (1, 12));
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
