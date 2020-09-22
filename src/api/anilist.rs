use anyhow::{bail, Context, Result};
use graphql_client::*;
use reqwest::{header, header::HeaderValue, Client};

use std::io::prelude::*;
use std::path::PathBuf;

struct Config {
    path: PathBuf,
}

impl Default for Config {
    #[cfg(not(windows))]
    fn default() -> Self {
        let mut path = PathBuf::from(dotenv::var("HOME").unwrap());
        path.push(".config/anime-dl/.anime-dl.cache");

        Self { path }
    }

    #[cfg(windows)]
    fn default() -> Self {
        let mut path = PathBuf::from(dotenv::var("HOMEPATH").unwrap());
        path.push(r"AppData\Roaming\anime-dl\.anime-dl.cache");

        Self { path }
    }
}

impl Config {
    fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    fn path(self, path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
        }
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

    fn clean(&self) -> Result<()> {
        std::fs::remove_file(&self.path).context("Unable to remove config file")
    }
}

pub struct AniListBuilder {
    client_id: Option<String>,
    token: Option<String>,
}

impl<'a> AniListBuilder {
    const AUTHORIZATION: &'a str = "Bearer ";
    const ACCEPT: &'a str = "application/json";
    const OAUTH_URL: &'a str = "https://anilist.co/api/v2/oauth/authorize?\
        response_type=token&client_id=";

    pub fn build(self) -> Result<AniList> {
        Ok(match self.client_id {
            None => bail!("No `CLIENT_ID` env varibale"),
            Some(client_id) => {
                let oauth_url = format!("{}{}", Self::OAUTH_URL, client_id);
                let config = Config::new();

                let token = match self.token {
                    Some(t) => t,
                    None => match config.load() {
                        Some(t) => t,
                        None => {
                            println!(
                                "For autentication go to: {}\n\nAnd paste token here:",
                                oauth_url
                            );
                            let mut line = String::new();
                            std::io::stdin().read_line(&mut line)?;

                            let token = line.trim().to_string();
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

                let client = Client::builder().default_headers(headers).build()?;

                AniList {
                    client,
                    anime_id: None,
                }
            }
        })
    }

    #[allow(dead_code)]
    pub fn token(self, token: &str) -> Self {
        Self {
            token: Some(token.to_string()),
            ..self
        }
    }

    #[allow(dead_code)]
    pub fn client_id(self, client_id: &str) -> Self {
        Self {
            client_id: Some(client_id.to_string()),
            ..self
        }
    }
}

impl Default for AniListBuilder {
    fn default() -> Self {
        dotenv::dotenv().ok();

        Self {
            client_id: dotenv::var("CLIENT_ID").ok(),
            token: None,
        }
    }
}

pub struct AniList {
    client: Client,
    anime_id: Option<u32>,
}

impl<'a> AniList {
    const REQUEST_URL: &'a str = "https://graphql.anilist.co";

    pub fn new() -> Option<Self> {
        AniListBuilder::default().build().ok()
    }

    #[allow(dead_code)]
    pub fn builder() -> AniListBuilder {
        AniListBuilder::default()
    }

    pub fn clean_cache() -> Result<()> {
        Config::default().clean()
    }

    pub fn id(self, id: u32) -> Self {
        Self {
            anime_id: Some(id),
            ..self
        }
    }

    pub async fn last_viewed(&self) -> Result<Option<u32>> {
        let q = ProgressQuery::build_query(progress_query::Variables {
            id: self.anime_id.map(|id| id as i64),
        });
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
        let c = Config::new().path(TEST_PATH);

        assert!(c.save(string).is_ok());
        let loaded_string = c.load().unwrap();
        assert_eq!(string, loaded_string);
        assert!(c.clean().is_ok());
    }

    #[test]
    #[should_panic(expected = "Unable to remove config file")]
    fn test_config_clean() {
        let c = Config::new().path(TEST_PATH);

        c.clean().unwrap()
    }
}
