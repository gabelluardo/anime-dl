use anyhow::{Result, anyhow, ensure};

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
        let text = response.split(Self::COOKIE_NAME).nth(1)?;
        let value = text.split(" ;  path=/").next()?.trim();

        let cookie = Self::COOKIE_NAME.to_owned() + value;

        Some(cookie)
    }

    async fn search(search: Search, client: Client) -> Result<Vec<Anime>> {
        let Search { id, string } = search;

        let search_results = {
            let keyword = string;
            let referrer = Self::REFERRER;

            let search_url = format!("{referrer}/search?keyword={keyword}");
            let search_page = selector::get_page(&client, &search_url).await?;

            let anime_list = selector::from("div.film-list");
            let Some(elem) = search_page.select(&anime_list).next() else {
                return Err(anyhow!("Request blocked, retry"));
            };

            let name = selector::from("a.name");

            let mut r: Vec<_> = elem
                .select(&name)
                .filter_map(|a| a.value().attr("href"))
                .map(|s| s.to_string())
                .collect();
            r.sort_unstable();

            r
        };

        ensure!(!search_results.is_empty(), "No anime found");

        let anilist = Anilist::new(id)?;

        let pool: Vec<_> = search_results
            .iter()
            .map(async |url| {
                let url = Self::REFERRER.to_string() + url;
                let page = selector::get_page(&client.clone(), &url).await?;

                let mut info = get_info(page)?;
                if let Some(id) = info.id
                    && let Some((_, l)) = anilist.get_progress(id).await
                {
                    info.last_watched = Some(l);
                }

                anyhow::Ok(info)
            })
            .collect();

        let stream: Vec<_> = stream::iter(pool).buffer_unordered(8).collect().await;
        let series: Vec<_> = stream.into_iter().filter_map(|a| a.ok()).collect();

        if let Some(id) = id
            && let Some(a) = series
                .iter()
                .find(|a| a.id == Some(id) && !a.name.contains("(ITA)"))
        {
            return Ok(vec![a.clone()]);
        }

        let mut series = series;
        if series.len() > 1 {
            Tui::select_series(&mut series)?;
        }

        Ok(series)
    }
}

fn get_info(page: Html) -> Result<Anime> {
    let Some(name) = get_name(&page) else {
        return Err(anyhow!("No name found"));
    };
    let Some(url) = get_url(&page) else {
        return Err(anyhow!("No url found"));
    };

    let id = get_id(&page);
    let range = get_range(&page).map(|e| e.into());

    Ok(Anime::new(&name, &url, id, range))
}

fn get_name(page: &Html) -> Option<String> {
    let h1 = selector::from(r#"h1[id="anime-title"]"#);

    let elem = page.select(&h1).next()?;
    let name = elem.first_child()?.value().as_text()?.to_string();

    Some(name)
}

fn get_url(page: &Html) -> Option<String> {
    let selectors = [
        r#"a[id="alternativeDownloadLink"]"#,
        r#"a[id="downloadLink"]"#,
        r#"a[id="customDownloadButton"]"#,
    ];

    for s in selectors {
        let a = selector::from(s);
        let elem = page.select(&a).next_back();

        if let Some(url) = elem.and_then(|e| e.value().attr("href")) {
            let url = url.replace("download-file.php?id=", "");
            return Some(url);
        }
    }

    None
}

fn get_id(page: &Html) -> Option<u32> {
    let btn = selector::from(r#"a[id="anilist-button"]"#);
    let href = page.select(&btn).next_back()?.value().attr("href")?;

    let url = Url::parse(href).ok()?;
    let value = url.path_segments()?.next_back()?;
    let id = value.parse().ok()?;

    Some(id)
}

fn get_range(page: &Html) -> Option<(u32, u32)> {
    let range = selector::from("div.range");
    let span = selector::from("span.rangetitle");

    if let Some(range) = page.select(&range).next()
        && let Some(_) = range.select(&span).next()
    {
        let spans: Vec<_> = range.select(&span).collect();

        return match spans.as_slice() {
            [first, .., last] => {
                let html = first.inner_html();
                let value = html.split_ascii_whitespace().next()?;
                let start = value.parse().ok()?;

                let html = last.inner_html();
                let value = html.split_ascii_whitespace().last()?;
                let end = value.parse().ok()?;

                Some((start, end))
            }
            _ => None,
        };
    }

    let ul = selector::from("ul.episodes");
    let a = selector::from("a");

    let elem = page.select(&ul).next()?;
    let values = elem.select(&a);
    let episodes: Vec<_> = values.filter_map(|a| a.inner_html().parse().ok()).collect();

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
