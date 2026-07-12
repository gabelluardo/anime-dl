pub use clap::Parser;

pub mod download;
pub mod stream;

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum Site {
    #[default]
    AW,
}

/// Efficient cli app for downloading anime
#[derive(Parser, Debug)]
#[command(author, version, about, arg_required_else_help = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    #[command(alias = "s")]
    Stream(stream::Args),
    #[command(alias = "d")]
    Download(download::Args),

    /// Delete app config
    Clean,
}

mod utils {
    use anyhow::{Result, anyhow};
    use reqwest::Client;

    use super::Site;
    use crate::{
        anilist::{Anilist, AnilistId},
        anime::Anime,
        archives::{AnimeWorld, Archive},
        error::RequestError,
        proxy::{ProxyConfig, get_random_proxy},
        scraper::{Scraper, ScraperConfig, Search},
        ui::Tui,
    };

    async fn get_from_watching_list(client: &Anilist) -> Result<Vec<Search>> {
        let Some(list) = client.get_watching_list().await else {
            return Err(anyhow!(RequestError::WatchingList));
        };

        let search = Tui::select_from_watching(&list)?
            .iter()
            .map(|info| {
                let id = Some(info.id());
                let string = info
                    .title()
                    .split_ascii_whitespace()
                    .take(3)
                    .enumerate()
                    .fold(String::new(), |mut acc, (index, part)| {
                        if index > 0 {
                            acc.push('+');
                        }
                        acc.push_str(part);
                        acc
                    });

                Search::new(string, id)
            })
            .collect();

        Ok(search)
    }

    fn get_from_input(entries: Vec<String>) -> Result<Vec<Search>> {
        let search = entries
            .join(" ")
            .split(',')
            .map(|s| s.trim().replace(' ', "+"))
            .map(|s| Search::new(s, None))
            .collect();

        Ok(search)
    }

    async fn search_site<T: Archive>(
        searches: &[Search],
        proxy: Option<String>,
        anilist_id: Option<AnilistId>,
    ) -> Result<(Vec<Anime>, &'static str)> {
        let cookie = T::extract_cookie().await;
        let config = ScraperConfig {
            proxy,
            cookie,
            anilist_id,
        };

        let anime = Scraper::new(config).search::<T>(searches).await?;

        Ok((anime, T::REFERRER))
    }

    pub async fn get_search_results(
        entries: Vec<String>,
        watching: bool,
        anilist_id: Option<AnilistId>,
        proxy: bool,
        site: Option<Site>,
    ) -> Result<(Vec<Anime>, &'static str)> {
        let anilist = Anilist::new(anilist_id)?;

        let searches = if watching || entries.is_empty() {
            get_from_watching_list(&anilist).await?
        } else {
            get_from_input(entries)?
        };

        let proxy = if proxy {
            let p = get_random_proxy(&Client::new(), ProxyConfig::new()).await?;
            Some(p)
        } else {
            None
        };

        let search_result = match site {
            Some(Site::AW) | None => {
                search_site::<AnimeWorld>(&searches, proxy, anilist_id).await?
            }
        };

        Ok(search_result)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::anime::AnimeId;
        use simple_test_case::test_case;

        #[test_case(vec!["bunny girl"], vec![("bunny+girl", None)]; "single entry")]
        #[test_case(
            vec!["bunny girl", "promare"],
            vec![("bunny+girl+promare", None)];
            "multiple entries joined"
        )]
        #[test_case(
            vec!["one piece,tokyo ghoul"],
            vec![("one+piece", None), ("tokyo+ghoul", None)];
            "comma separated entries")]
        #[test_case(vec!["  spaces  "], vec![("spaces", None)]; "trim spaces")]
        #[test]
        fn test_get_from_input(entries: Vec<&str>, expected: Vec<(&str, Option<AnimeId>)>) {
            let entries: Vec<String> = entries.into_iter().map(String::from).collect();
            let result = get_from_input(entries).unwrap();

            assert_eq!(result.len(), expected.len());
            for (got, (exp_str, exp_id)) in result.iter().zip(expected.iter()) {
                assert_eq!(got.string, *exp_str);
                assert_eq!(got.id, *exp_id);
            }
        }

        #[test_case(Site::AW; "aw is default")]
        #[test]
        fn test_site_default(site: Site) {
            assert!(matches!(site, Site::AW));
        }

        #[test_case(
            vec!["one piece,tokyo ghoul,fullmetal"],
            vec![("one+piece", None), ("tokyo+ghoul", None), ("fullmetal", None)];
            "three comma separated"
        )]
        #[test_case(
            vec!["  spaced  ,  another  "],
            vec![("spaced", None), ("another", None)];
            "comma separated with spaces"
        )]
        #[test_case(
            vec!["single"],
            vec![("single", None)];
            "single word"
        )]
        #[test_case(
            vec!["two words"],
            vec![("two+words", None)];
            "two words joined"
        )]
        #[test]
        fn test_get_from_input_extra(entries: Vec<&str>, expected: Vec<(&str, Option<AnimeId>)>) {
            let entries: Vec<String> = entries.into_iter().map(String::from).collect();
            let result = get_from_input(entries).unwrap();

            assert_eq!(result.len(), expected.len());
            for (got, (exp_str, exp_id)) in result.iter().zip(expected.iter()) {
                assert_eq!(got.string, *exp_str);
                assert_eq!(got.id, *exp_id);
            }
        }

        #[test_case(vec![("test", None)], 1; "single entry no id")]
        #[test_case(vec![("test+other", None)], 1; "joined entries")]
        #[test]
        fn test_get_from_input_preserves_search_structure(
            expected: Vec<(&str, Option<AnimeId>)>,
            exp_len: usize,
        ) {
            let entries: Vec<String> = expected.iter().map(|(s, _)| String::from(*s)).collect();
            let result = get_from_input(entries).unwrap();
            assert_eq!(result.len(), exp_len);
            for (got, (exp_str, exp_id)) in result.iter().zip(expected.iter()) {
                assert_eq!(got.string, *exp_str);
                assert_eq!(got.id, *exp_id);
            }
        }
    }
}
