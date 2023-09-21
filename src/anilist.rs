use std::path::PathBuf;
use std::{fs, io::Read, io::Write};

use anyhow::{Context, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest::{header, header::HeaderValue, Client};

use crate::errors::SystemError;
use crate::tui;

struct Config(PathBuf);

impl Config {
    #[cfg(not(windows))]
    fn new() -> Self {
        let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_default());
        path.push(".config/anime-dl/.anime-dl.cache");
        Self(path)
    }

    #[cfg(windows)]
    fn new() -> Self {
        let mut path = PathBuf::from(std::env::var("HOMEPATH").unwrap_or_default());
        path.push(r"AppData\Roaming\anime-dl\.anime-dl.cache");
        Self(path)
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
    schema_path = "assets/anilist_schema.graphql",
    query_path = "assets/progress_query.graphql"
)]
struct ProgressQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "assets/anilist_schema.graphql",
    query_path = "assets/watching_query.graphql",
    response_derives = "Clone"
)]
struct WatchingQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "assets/anilist_schema.graphql",
    query_path = "assets/user_query.graphql"
)]
struct UserQuery;

#[derive(Default, Debug)]
pub struct AniList(Client);

impl AniList {
    pub fn new(client_id: Option<u32>) -> Self {
        let client_id = client_id.unwrap_or(4047);
        let config = Config::new();
        let oauth_url = format!(
            "https://anilist.co/api/v2/oauth/authorize?response_type=token&client_id={client_id}"
        );
        let token = match config.load() {
            Ok(t) => Some(t),
            _ => oauth_token(&oauth_url, &config),
        };

        let mut client = Client::new();
        if let Some(token) = token {
            let mut headers = header::HeaderMap::new();
            headers.insert(
                header::AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
            );
            headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );

            client = Client::builder().default_headers(headers).build().unwrap();
        }

        Self(client)
    }

    pub fn clean_cache() -> Result<()> {
        Config::new().clean()
    }

    pub async fn last_watched(&self, anime_id: Option<u32>) -> Option<u32> {
        let url = "https://graphql.anilist.co";
        let variables = progress_query::Variables {
            id: anime_id.map(|u| u as i64),
        };
        let query = ProgressQuery::build_query(variables);
        let res = self.0.post(url).json(&query).send().await.ok()?;
        let response_body = res
            .json::<Response<progress_query::ResponseData>>()
            .await
            .ok()?;

        response_body
            .data?
            .media?
            .media_list_entry?
            .progress
            .map(|p| p as u32)
    }

    pub async fn get_watching_list(&self) -> Option<Vec<String>> {
        let url = "https://graphql.anilist.co";
        let query = UserQuery::build_query(user_query::Variables);
        let res = self.0.post(url).json(&query).send().await.ok()?;
        let response_body = res
            .json::<Response<user_query::ResponseData>>()
            .await
            .ok()?;
        let user_id = response_body.data?.viewer.map(|d| d.id);

        let variables = watching_query::Variables { id: user_id };
        let query = WatchingQuery::build_query(variables);

        let res = self.0.post(url).json(&query).send().await.ok()?;
        let response_body = res
            .json::<Response<watching_query::ResponseData>>()
            .await
            .ok()?;

        let list = response_body.data?.media_list_collection?.lists?[0]
            .clone()?
            .entries?
            .into_iter()
            .filter_map(|e| {
                e.and_then(|m| m.media)
                    .and_then(|m| m.title)
                    .and_then(|t| t.romaji)
            })
            .collect::<Vec<_>>();

        Some(list)
    }
}

fn oauth_token(oauth_url: &str, config: &Config) -> Option<String> {
    let token = tui::get_token(oauth_url).ok()?;
    config.save(&token).ok()?;

    Some(token)
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use super::*;

    lazy_static! {
        static ref config: Config = Config(PathBuf::from("/tmp/test.cache"));
        static ref config_panic: Config = Config(PathBuf::from("/tmp/test2.cache"));
    }

    #[tokio::test]
    async fn test_watching_list() {
        // todo: decidere se lascirare questo test

        let al = AniList::new(Some(4047));

        let list = al.get_watching_list().await;

        dbg!(list);
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
