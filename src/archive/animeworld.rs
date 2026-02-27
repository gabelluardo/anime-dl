use anyhow::{Context, Result, anyhow, ensure};

use futures::stream::StreamExt;
use reqwest::{Client, Url};
use tokio_stream as stream;

use super::{Archive, Html, selector};
use crate::anilist::Anilist;
use crate::anime::Anime;
use crate::scraper::Search;
use crate::tui::Tui;

pub struct AnimeWorld;
impl Archive for AnimeWorld {
    const REFERRER: &'static str = "https://www.animeworld.ac";
    const COOKIE_NAME: &'static str = "SecurityAW";

    async fn extract_cookie() -> Option<String> {
        let response = reqwest::get(Self::REFERRER).await.ok()?.text().await.ok()?;

        response
            .split(Self::COOKIE_NAME)
            .nth(1)
            .and_then(|s| s.split(" ;  path=/").next())
            .map(|s| Self::COOKIE_NAME.to_owned() + s.trim())
    }

    async fn search(search: Search, client: Client) -> Result<Vec<Anime>> {
        let search_results = {
            let keyword = &search.string;
            let referrer = Self::REFERRER;

            let search_url = format!("{referrer}/search?keyword={keyword}");
            let search_page = selector::get_page(&client, &search_url).await?;

            let anime_list = selector::from("div.film-list");
            let name = selector::from("a.name");

            let elem = search_page
                .select(&anime_list)
                .next()
                .context("Request blocked, retry")?;

            let mut r: Vec<_> = elem
                .select(&name)
                .map(|a| a.value().attr("href").expect("No link found").to_string())
                .collect();
            r.sort_unstable();

            r
        };

        ensure!(!search_results.is_empty(), "No anime found");

        let anilist = Anilist::new(search.id)?;

        let pool: Vec<_> = search_results
            .iter()
            .map(async |url| {
                let url = Self::REFERRER.to_string() + url;
                let page = selector::get_page(&client.clone(), &url).await?;

                let mut info = get_info(page)?;
                if let Some(id) = info.id {
                    info.last_watched = anilist.get_progress(id).await.map(|(_, l)| l);
                }

                anyhow::Ok::<Anime>(info)
            })
            .collect();

        let stream: Vec<_> = stream::iter(pool).buffer_unordered(8).collect().await;
        let mut anime = stream.into_iter().filter_map(|a| a.ok());

        if let Some(id) = search.id
            && let Some(anime) = anime.find(|a| a.id == Some(id) && !a.name.contains("(ITA)"))
        {
            return Ok(vec![anime]);
        }

        let series = {
            let mut s: Vec<_> = anime.collect();
            if s.len() > 1 {
                Tui::select_series(&mut s)?;
            }

            s
        };

        Ok(series)
    }
}

fn get_info(page: Html) -> Result<Anime> {
    let range = get_range(&page);
    let id = get_id(&page);
    let name = get_name(&page)?;
    let url = get_url(&page)?;

    Ok(Anime::new(&name, &url, id, range.map(|e| e.into())))
}

fn get_name(page: &Html) -> Result<String> {
    let h1 = selector::from(r#"h1[id="anime-title"]"#);

    page.select(&h1)
        .next()
        .and_then(|e| e.first_child().and_then(|a| a.value().as_text()))
        .map(|t| t.to_string())
        .context("No name found")
}

fn get_url(page: &Html) -> Result<String> {
    let a = selector::from(r#"a[id="alternativeDownloadLink"]"#);
    let mut url = page
        .select(&a)
        .next_back()
        .and_then(|a| a.value().attr("href"));

    if url.is_none_or(|u| u.is_empty()) {
        let a = selector::from(r#"a[id="downloadLink"]"#);
        url = page
            .select(&a)
            .next_back()
            .and_then(|a| a.value().attr("href"))
    }

    if url.is_none_or(|u| u.is_empty()) {
        let a = selector::from(r#"a[id="customDownloadButton"]"#);
        url = page
            .select(&a)
            .next_back()
            .and_then(|a| a.value().attr("href"))
    }

    let url = url
        .map(|u| u.replace("download-file.php?id=", ""))
        .ok_or(anyhow!("No url found"))?;

    Ok(url)
}

fn get_id(page: &Html) -> Option<u32> {
    let btn = selector::from(r#"a[id="anilist-button"]"#);

    page.select(&btn)
        .next_back()
        .and_then(|a| a.value().attr("href"))
        .and_then(|u| {
            Url::parse(u)
                .unwrap()
                .path_segments()
                .and_then(|mut s| s.next_back())
                .and_then(|s| s.parse::<u32>().ok())
        })
}

fn get_range(page: &Html) -> Option<(u32, u32)> {
    let range = selector::from("div.range");
    let span = selector::from("span.rangetitle");

    if let Some(range) = page.select(&range).next()
        && let Some(_) = range.select(&span).next()
    {
        let spans: Vec<_> = range.select(&span).collect();

        let range = match spans.as_slice() {
            [first, .., last] => {
                let start = first
                    .inner_html()
                    .split_ascii_whitespace()
                    .next()?
                    .parse::<u32>()
                    .ok()?;

                let end = last
                    .inner_html()
                    .split_ascii_whitespace()
                    .last()?
                    .parse::<u32>()
                    .ok()?;

                Some((start, end))
            }
            _ => None,
        };

        return range;
    }

    let ul = selector::from("ul.episodes");
    let a = selector::from("a");

    let episodes = page
        .select(&ul)
        .next()?
        .select(&a)
        .filter_map(|a| a.inner_html().parse::<u32>().ok())
        .collect::<Vec<_>>();

    match episodes.as_slice() {
        [start, .., end] => Some((*start, *end)),
        [single] => Some((*single, *single)),
        [] => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Url;

    pub fn get_url(raw_url: &str) -> String {
        Url::parse(raw_url)
            .unwrap()
            .path_segments()
            .and_then(|mut s| s.next_back())
            .unwrap()
            .to_owned()
    }

    mod animeworld {
        use crate::scraper::{Scraper, ScraperConfig};

        use super::*;

        #[test]
        fn test_parse_episodes() {
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
            let episodes = get_range(&fragment).unwrap();

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
            let episodes = get_range(&fragment).unwrap();

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
            let episodes = get_range(&fragment).unwrap();

            assert_eq!(episodes, (1, 12));
        }

        #[tokio::test]
        #[ignore]
        async fn test_remote() {
            let file = "SeishunButaYarouWaBunnyGirlSenpaiNoYumeWoMinai_Ep_01_SUB_ITA.mp4";
            let cookie = AnimeWorld::extract_cookie().await;
            let config = ScraperConfig {
                cookie,
                proxy: None,
            };

            let scraper = Scraper::new(config);

            let search = Search {
                string: "bunny girl".into(),
                id: None,
            };

            let anime = AnimeWorld::search(search, scraper.client()).await.unwrap();

            let info = get_url(&anime.first().unwrap().origin);

            assert_eq!(file, info)
        }
    }
}
