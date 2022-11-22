use std::path::PathBuf;
use std::{fs, io::Read, io::Write};

use anyhow::{Context, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest::{header, header::HeaderValue, Client};

use crate::errors::SystemError;
use crate::utils::tui;

struct Config(PathBuf);

impl Default for Config {
    #[cfg(not(windows))]
    fn default() -> Self {
        let mut path = PathBuf::from(std::env::var("HOME").unwrap());
        path.push(".config/anime-dl/.anime-dl.cache");

        Self(path)
    }

    #[cfg(windows)]
    fn default() -> Self {
        let mut path = PathBuf::from(std::env::var("HOMEPATH").unwrap());
        path.push(r"AppData\Roaming\anime-dl\.anime-dl.cache");

        Self(path)
    }
}

impl Config {
    fn new() -> Self {
        Self::default()
    }

    fn clean(&self) -> Result<()> {
        fs::remove_file(&self.0).context(SystemError::FsRemove)
    }

    fn load(&self) -> Result<String> {
        let file = fs::OpenOptions::new().read(true).open(&self.0);

        file.map(|mut f| {
            let mut contents = String::new();

            f.read_to_string(&mut contents).ok();

            contents
        })
        .context(SystemError::FsLoad)
    }

    fn save(&self, token: &str) -> Result<()> {
        let path = &self.0;

        if !path.exists() {
            fs::create_dir_all(path.parent().unwrap())?;
        }

        let mut buf = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        buf.write_all(token.as_bytes())
            .context(SystemError::FsWrite)
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/utils/assets/anilist_schema.graphql",
    query_path = "src/utils/assets/progress_query.graphql",
    response_derives = "Debug"
)]
pub struct ProgressQuery;

#[derive(Default, Debug)]
pub struct AniList(Client);

impl AniList {
    pub fn new(client_id: Option<u32>) -> Result<Self> {
        let client_id = client_id.unwrap_or(4047);
        let config = Config::new();

        let oauth_url = format!(
            "https://anilist.co/api/v2/oauth/authorize?response_type=token&client_id={client_id}"
        );

        let token = match config.load() {
            Ok(t) => t,
            _ => {
                let token = tui::get_token(&oauth_url)?;
                config.save(&token)?;
                token
            }
        };

        let mut headers = header::HeaderMap::new();
        let auth = HeaderValue::from_str(&format!("Bearer {token}"))?;
        let application = HeaderValue::from_static("application/json");

        headers.insert(header::AUTHORIZATION, auth);
        headers.insert(header::ACCEPT, application.clone());
        headers.insert(header::CONTENT_TYPE, application);

        let client = Self(Client::builder().default_headers(headers).build()?);

        Ok(client)
    }

    pub fn clean_cache() -> Result<()> {
        Config::default().clean()
    }

    pub async fn last_viewed(&self, anime_id: Option<u32>) -> Result<Option<u32>> {
        let endpoint = "https://graphql.anilist.co";

        let q = ProgressQuery::build_query(progress_query::Variables {
            id: anime_id.map(|u| u as i64),
        });
        let res = self.0.post(endpoint).json(&q).send().await?;
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

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use super::*;

    lazy_static! {
        static ref config: Config = Config(PathBuf::from("/tmp/test.cache"));
        static ref config_panic: Config = Config(PathBuf::from("/tmp/test2.cache"));
    }

    #[test]
    fn test_write_config() {
        let string = "asdfasdfasdf";

        let res = config.save(string);
        assert!(res.is_ok());
        assert!(config.0.is_file());

        let res = config.load();
        assert!(res.is_ok());
        assert_eq!(string, res.unwrap());

        let res = config.clean();
        assert!(res.is_ok());
    }

    #[test]
    #[should_panic]
    fn test_clean_config_panic() {
        config_panic.clean().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_load_config_panic() {
        config_panic.load().unwrap();
    }
}
