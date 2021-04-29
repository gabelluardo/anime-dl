use std::path::PathBuf;

use graphql_client::{GraphQLQuery, Response};
use reqwest::{header, header::HeaderValue, Client};
use tokio::{fs, io::AsyncReadExt, io::AsyncWriteExt};

use crate::errors::{Error, Result};
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

    async fn clean(&self) -> Result<()> {
        fs::remove_file(&self.0).await.map_err(|_| Error::FsRemove)
    }

    async fn load(&self) -> Option<String> {
        let file = fs::OpenOptions::new().read(true).open(&self.0).await;

        match file {
            Ok(mut f) => {
                let mut contents = String::new();
                f.read_to_string(&mut contents).await.ok();
                Some(contents)
            }
            _ => None,
        }
    }

    async fn save(&self, token: &str) -> Result<()> {
        if !self.0.exists() {
            fs::create_dir_all(&self.0.parent().unwrap()).await?;
        }

        let mut buf = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.0)
            .await?;

        buf.write_all(token.as_bytes())
            .await
            .map_err(|_| Error::FsWrite)
    }
}

#[derive(Default)]
pub struct AniListBuilder {
    anime_id: Option<u32>,
    client_id: Option<u32>,
}

impl AniListBuilder {
    pub fn anime_id(mut self, anime_id: Option<u32>) -> Self {
        self.anime_id = anime_id;
        self
    }

    pub fn client_id(mut self, client_id: Option<u32>) -> Self {
        self.client_id = client_id;
        self
    }

    pub async fn build(self) -> Result<AniList> {
        let client_id = self.client_id.ok_or(Error::EnvNotFound)?;
        let config = Config::new();

        let oauth_url = format!(
            "https://anilist.co/api/v2/oauth/authorize?response_type=token&client_id={}",
            client_id
        );

        let token = match config.load().await {
            Some(t) => t,
            _ => {
                let token = tui::get_token(&oauth_url).await?;
                config.save(&token).await?;
                token
            }
        };

        let mut headers = header::HeaderMap::new();
        let auth = HeaderValue::from_str(&format!("Bearer {}", token))
            .map_err(|e| Error::InvalidToken(e))?;
        let application = HeaderValue::from_static("application/json");

        headers.insert(header::AUTHORIZATION, auth);
        headers.insert(header::ACCEPT, application.clone());
        headers.insert(header::CONTENT_TYPE, application);

        let anime_id = self.anime_id.map(|id| id as i64);
        let client = Client::builder().default_headers(headers).build()?;

        Ok(AniList { anime_id, client })
    }
}

pub struct AniList {
    client: Client,
    anime_id: Option<i64>,
}

impl AniList {
    pub fn builder() -> AniListBuilder {
        AniListBuilder::default()
    }

    pub async fn clean_cache() -> Result<()> {
        Config::default().clean().await
    }

    pub async fn last_viewed(&self) -> Result<Option<u32>> {
        let endpoint = "https://graphql.anilist.co";

        let q = ProgressQuery::build_query(progress_query::Variables { id: self.anime_id });
        let res = self.client.post(endpoint).json(&q).send().await?;
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

    const PATH_STR: &str = "/tmp/test.cache";
    const PATH_STR_PANIC: &str = "/tmp/test2.cache";

    #[tokio::test]
    async fn test_write_config() {
        let string = "asdfasdfasdf";
        let path = PathBuf::from(PATH_STR);
        let c = Config(path.clone());

        let res = c.save(string).await;
        assert!(res.is_ok());
        assert!(path.is_file());

        let res = c.load().await;
        assert!(res.is_some());
        assert_eq!(string, res.unwrap());

        let res = c.clean().await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    #[should_panic]
    async fn test_clean_config_panic() {
        let path = PathBuf::from(PATH_STR_PANIC);
        let c = Config(path);

        c.clean().await.unwrap()
    }
}
