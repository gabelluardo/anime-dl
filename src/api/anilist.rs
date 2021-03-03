use crate::utils::tui;

use anyhow::{bail, Context, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest::{header, header::HeaderValue, Client};

use std::io::prelude::*;
use std::path::PathBuf;

struct Config {
    path: PathBuf,
}

impl Default for Config {
    #[cfg(not(windows))]
    fn default() -> Self {
        let mut path = PathBuf::from(std::env::var("HOME").unwrap());
        path.push(".config/anime-dl/.anime-dl.cache");

        Self { path }
    }

    #[cfg(windows)]
    fn default() -> Self {
        let mut path = PathBuf::from(std::env::var("HOMEPATH").unwrap());
        path.push(r"AppData\Roaming\anime-dl\.anime-dl.cache");

        Self { path }
    }
}

impl Config {
    fn new() -> Self {
        Self::default()
    }

    fn clean(&self) -> Result<()> {
        std::fs::remove_file(&self.path).context("Unable to remove config file")
    }

    fn load(&self) -> Option<String> {
        std::fs::OpenOptions::new()
            .read(true)
            .open(&self.path)
            .ok()
            .map(|mut f| {
                let mut contents = String::new();
                f.read_to_string(&mut contents).ok();
                contents
            })
    }

    fn save(&self, token: &str) -> Result<()> {
        let mut dirs = self.path.clone();
        dirs.pop();

        if !self.path.exists() {
            std::fs::create_dir_all(&dirs)?;
        }
        let mut buf = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;

        buf.write_all(token.as_bytes())
            .context("Unable to write config file")
    }
}

#[derive(Default)]
pub struct AniListBuilder {
    anime_id: Option<u32>,
    client_id: Option<u32>,
    token: Option<String>,
}

impl<'a> AniListBuilder {
    const AUTHORIZATION: &'a str = "Bearer ";
    const ACCEPT: &'a str = "application/json";
    const OAUTH_URL: &'a str = "https://anilist.co/api/v2/oauth/authorize?\
        response_type=token&client_id=";

    pub fn anime_id(mut self, anime_id: Option<u32>) -> Self {
        self.anime_id = anime_id;
        self
    }

    pub fn client_id(mut self, client_id: Option<u32>) -> Self {
        self.client_id = client_id;
        self
    }

    pub fn build(self) -> Result<AniList> {
        match self.client_id {
            Some(client_id) => {
                let oauth_url = format!("{}{}", Self::OAUTH_URL, client_id);
                let config = Config::new();

                let token = match self.token {
                    Some(t) => t,
                    None => match config.load() {
                        Some(t) => t,
                        None => {
                            let token = tui::get_token(&oauth_url)?;
                            config.save(&token)?;
                            token
                        }
                    },
                };

                let mut headers = header::HeaderMap::new();
                let auth = HeaderValue::from_str(&format!("{}{}", Self::AUTHORIZATION, token))?;

                headers.insert(header::AUTHORIZATION, auth);
                headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
                headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(Self::ACCEPT));

                Ok(AniList {
                    anime_id: self.anime_id.map(|id| id as i64),
                    client: Client::builder().default_headers(headers).build()?,
                })
            }
            _ => bail!("No `ANIMEDL_ID` env varibale"),
        }
    }
}

pub struct AniList {
    client: Client,
    anime_id: Option<i64>,
}

impl<'a> AniList {
    const REQUEST_URL: &'a str = "https://graphql.anilist.co";

    pub fn builder() -> AniListBuilder {
        AniListBuilder::default()
    }

    pub fn clean_cache() -> Result<()> {
        Config::default().clean()
    }

    pub async fn last_viewed(&self) -> Result<Option<u32>> {
        let q = ProgressQuery::build_query(progress_query::Variables { id: self.anime_id });
        let res = self.client.post(Self::REQUEST_URL).json(&q).send().await?;
        let response_body: Response<progress_query::ResponseData> = res.json().await?;

        let data = response_body
            .data
            .and_then(|d| d.media)
            .and_then(|m| m.media_list_entry)
            .and_then(|l| l.progress)
            .map(|p| p as u32);

        Ok(data)
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/api/assets/anilist_schema.graphql",
    query_path = "src/api/assets/progress_query.graphql",
    response_derives = "Debug"
)]
pub struct ProgressQuery;

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PATH: &str = "test.cache";

    #[test]
    fn test_config() {
        let string = "asdfasdfasdf";
        let c = Config {
            path: PathBuf::from(TEST_PATH),
        };

        assert!(c.save(string).is_ok());

        let loaded_string = c.load();
        assert!(loaded_string.is_some());
        assert_eq!(string, loaded_string.unwrap());

        assert!(c.clean().is_ok());
    }

    #[test]
    #[should_panic(expected = "Unable to remove config file")]
    fn test_config_clean() {
        let c = Config {
            path: PathBuf::from(TEST_PATH),
        };

        c.clean().unwrap()
    }
}
