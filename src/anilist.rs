use std::str::FromStr;

use anyhow::{Result, anyhow};

use derive_more::{Display, From, Into};
use graphql_client::{GraphQLQuery, Response};
use reqwest::{Client, header, header::HeaderValue};

use crate::{
    anime::{AnimeId, EpisodeId},
    config,
    ui::Tui,
};

const ENDPOINT: &str = "https://graphql.anilist.co";

/// Identifies an AniList API client (OAuth application ID).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From, Display)]
#[display("{_0}")]
pub struct AnilistId(u32);

impl FromStr for AnilistId {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(Self)
    }
}

#[derive(Debug)]
pub struct WatchingAnime {
    watched: i64,
    id: i64,
    title: String,
}

impl WatchingAnime {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn watched(&self) -> u32 {
        self.watched.max(0) as u32
    }

    pub fn id(&self) -> AnimeId {
        self.id.into()
    }
}

pub struct Progress {
    episodes: i64,
    progress: i64,
}

impl Progress {
    pub fn latest(&self) -> EpisodeId {
        self.progress.into()
    }

    fn has_seen(&self, n: EpisodeId) -> bool {
        self.progress >= n.into()
    }

    fn is_last(&self, n: EpisodeId) -> bool {
        self.episodes <= n.into()
    }
}

#[derive(GraphQLQuery, Debug)]
#[graphql(
    schema_path = "schema/anilist_schema.json",
    query_path = "schema/progress_query.graphql"
)]
struct ProgressQuery;

impl ProgressQuery {
    async fn get(client: &Client, id: AnimeId) -> Option<Progress> {
        let id = Some(id.into());
        let query = Self::build_query(progress_query::Variables { id });
        let response = client.post(ENDPOINT).json(&query).send().await.ok()?;
        let json = response
            .json::<Response<progress_query::ResponseData>>()
            .await
            .ok()?;

        let media = json.data?.media?;

        let episodes = media.episodes?;
        let progress = media.media_list_entry?.progress?;

        Some(Progress { episodes, progress })
    }
}

#[derive(GraphQLQuery, Debug)]
#[graphql(
    schema_path = "schema/anilist_schema.json",
    query_path = "schema/progress_mutation.graphql"
)]
struct ProgressMutation;

impl ProgressMutation {
    async fn put(
        client: &Client,
        id: AnimeId,
        progress: EpisodeId,
        status: progress_mutation::MediaListStatus,
    ) -> Result<()> {
        let variables = progress_mutation::Variables {
            status: Some(status),
            id: Some(id.into()),
            progress: Some(progress.into()),
        };

        let query = ProgressMutation::build_query(variables);
        client
            .post(ENDPOINT)
            .json(&query)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(GraphQLQuery, Debug)]
#[graphql(
    schema_path = "schema/anilist_schema.json",
    query_path = "schema/watching_query.graphql"
)]
struct WatchingQuery;

impl WatchingQuery {
    async fn get(client: &Client, id: UserId) -> Option<Vec<WatchingAnime>> {
        let id = Some(id.into());
        let variables = watching_query::Variables { id };
        let query = WatchingQuery::build_query(variables);
        let response = client.post(ENDPOINT).json(&query).send().await.ok()?;
        let json = response
            .json::<Response<watching_query::ResponseData>>()
            .await
            .ok()?;

        let collection = json
            .data?
            .media_list_collection?
            .lists?
            .get_mut(0)?
            .take()?;

        let mut list: Vec<_> = collection
            .entries?
            .into_iter()
            .filter_map(|collection| {
                let progress = collection.as_ref()?.progress?;
                let media = collection?.media?;

                let watched = match media.next_airing_episode {
                    Some(airing) => airing.episode - (progress + 1),
                    None => media.episodes? - progress,
                };

                let title = media.title?.romaji?;
                let id = media.id;

                Some(WatchingAnime { id, title, watched })
            })
            .collect();

        list.sort_by(|a, b| a.title.cmp(&b.title));

        Some(list)
    }
}

#[derive(Clone, Copy, Debug, From, Into)]
struct UserId(i64);

#[derive(GraphQLQuery, Debug)]
#[graphql(
    schema_path = "schema/anilist_schema.json",
    query_path = "schema/user_query.graphql"
)]
struct UserQuery;

impl UserQuery {
    async fn get(client: &Client) -> Option<UserId> {
        let query = UserQuery::build_query(user_query::Variables);
        let response = client.post(ENDPOINT).json(&query).send().await.ok()?;
        let json = response
            .json::<Response<user_query::ResponseData>>()
            .await
            .ok()?;

        json.data?.viewer.map(|d| d.id.into())
    }
}

#[derive(Default, Debug)]
pub struct Anilist {
    client: Client,
}

impl Anilist {
    pub fn new(client_id: Option<AnilistId>) -> Result<Self> {
        let client_id = client_id.unwrap_or(AnilistId(4047));
        let token = config::load("token").map_or_else(|| oauth_token(client_id), Ok)?;

        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| anyhow!("Invalid token"))?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|_| anyhow!("Unable to create client"))?;

        Ok(Self { client })
    }

    pub async fn get_watching_list(&self) -> Option<Vec<WatchingAnime>> {
        let user_id = UserQuery::get(&self.client).await?;
        let list = WatchingQuery::get(&self.client, user_id).await?;

        Some(list)
    }

    pub async fn get_progress(&self, id: AnimeId) -> Option<Progress> {
        ProgressQuery::get(&self.client, id).await
    }

    pub async fn update(&mut self, id: AnimeId, number: EpisodeId) -> Result<()> {
        use progress_mutation::MediaListStatus;

        let progress = self.get_progress(id).await;
        let status = match progress {
            Some(p) if p.has_seen(number) => return Ok(()),
            Some(p) if p.is_last(number) => MediaListStatus::COMPLETED,
            None | Some(_) => MediaListStatus::CURRENT,
        };

        ProgressMutation::put(&self.client, id, number, status).await?;

        Ok(())
    }
}

fn oauth_token(client_id: AnilistId) -> Result<String> {
    let url = format!(
        "https://anilist.co/api/v2/oauth/authorize?response_type=token&client_id={client_id}"
    );
    let token = Tui::get_token(&url);

    config::save("token", &token)?;

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case("42", 42; "valid id")]
    #[test_case("0", 0; "zero")]
    #[test_case("999999", 999999; "large id")]
    #[test]
    fn test_anilist_id_from_str(s: &str, expected: u32) {
        assert_eq!(AnilistId::from_str(s).unwrap(), AnilistId(expected));
    }

    #[test_case("abc"; "non numeric")]
    #[test_case("-1"; "negative")]
    #[test_case("" ; "empty")]
    #[test]
    fn test_anilist_id_from_str_err(s: &str) {
        assert!(AnilistId::from_str(s).is_err());
    }

    #[test_case(42, "42"; "display id")]
    #[test_case(0, "0"; "display zero")]
    #[test]
    fn test_anilist_id_display(input: u32, expected: &str) {
        assert_eq!(AnilistId(input).to_string(), expected);
    }

    #[test_case(5, 5; "positive watched")]
    #[test_case(0, 0; "zero watched")]
    #[test_case(-3, 0; "negative watched clamps to zero")]
    #[test]
    fn test_watching_anime_watched(watched: i64, expected: u32) {
        let anime = WatchingAnime {
            watched,
            id: 1,
            title: "Test".into(),
        };
        assert_eq!(anime.watched(), expected);
    }

    #[test_case("My Anime", "My Anime"; "standard title")]
    #[test_case("", ""; "empty title")]
    #[test_case("Special!@#$%", "Special!@#$%"; "special chars")]
    #[test]
    fn test_watching_anime_title(title: &str, expected: &str) {
        let anime = WatchingAnime {
            watched: 0,
            id: 1,
            title: title.into(),
        };
        assert_eq!(anime.title(), expected);
    }

    #[test_case(42, 42; "positive id")]
    #[test_case(0, 0; "zero id")]
    #[test]
    fn test_watching_anime_id(id: i64, expected: u32) {
        let anime = WatchingAnime {
            watched: 0,
            id,
            title: "Test".into(),
        };
        assert_eq!(anime.id(), AnimeId(expected));
    }

    #[test_case(5, 5; "progress five")]
    #[test_case(0, 0; "progress zero")]
    #[test_case(100, 100; "progress hundred")]
    #[test]
    fn test_progress_latest(progress: i64, expected: u32) {
        let p = Progress {
            episodes: 12,
            progress,
        };
        assert_eq!(p.latest(), EpisodeId(expected));
    }

    #[test_case(5, 3, true; "seen episode three")]
    #[test_case(5, 5, true; "seen episode five")]
    #[test_case(5, 6, false; "not seen episode six")]
    #[test_case(0, 0, true; "zero seen zero")]
    #[test]
    fn test_progress_has_seen(progress: i64, ep: u32, expected: bool) {
        let p = Progress {
            episodes: 12,
            progress,
        };
        assert_eq!(p.has_seen(EpisodeId(ep)), expected);
    }

    #[test_case(12, 12, true; "is last episode")]
    #[test_case(12, 13, true; "beyond last episode")]
    #[test_case(12, 11, false; "before last episode")]
    #[test_case(12, 5, false; "well before last")]
    #[test]
    fn test_progress_is_last(episodes: i64, ep: u32, expected: bool) {
        let p = Progress {
            episodes,
            progress: 0,
        };
        assert_eq!(p.is_last(EpisodeId(ep)), expected);
    }

    #[test_case(42, 42; "from u32")]
    #[test_case(0, 0; "from zero")]
    #[test]
    fn test_anilist_id_from(input: u32, expected: u32) {
        let id: AnilistId = input.into();
        assert_eq!(id, AnilistId(expected));
    }

    #[test_case(42i64, 42; "from i64 positive")]
    #[test_case(0, 0; "from i64 zero")]
    #[test_case(-1, -1; "from i64 negative")]
    #[test]
    fn test_user_id_from(input: i64, expected: i64) {
        let id: UserId = input.into();
        let result: i64 = id.into();
        assert_eq!(result, expected);
    }

    #[test_case(5, 5, true; "seen equal progress")]
    #[test_case(5, 10, false; "not seen beyond progress")]
    #[test_case(0, 0, true; "zero seen zero")]
    #[test]
    fn test_progress_has_seen_edge(progress: i64, ep: u32, expected: bool) {
        let p = Progress {
            episodes: 12,
            progress,
        };
        assert_eq!(p.has_seen(EpisodeId(ep)), expected);
    }

    #[test_case(1, 1, true; "single episode is last")]
    #[test_case(0, 0, true; "zero episodes zero is last")]
    #[test_case(100, 50, false; "well before last")]
    #[test]
    fn test_progress_is_last_edge(episodes: i64, ep: u32, expected: bool) {
        let p = Progress {
            episodes,
            progress: 0,
        };
        assert_eq!(p.is_last(EpisodeId(ep)), expected);
    }

    #[test_case(-5, 0; "negative watched clamps")]
    #[test_case(100, 100; "large watched")]
    #[test]
    fn test_watching_anime_watched_edge(watched: i64, expected: u32) {
        let anime = WatchingAnime {
            watched,
            id: 1,
            title: "Test".into(),
        };
        assert_eq!(anime.watched(), expected);
    }

    #[test_case(3, 42, "Test Anime", 3, AnimeId(42); "all methods")]
    #[test_case(0, 1, "A", 0, AnimeId(1); "zero watched")]
    #[test]
    fn test_watching_anime_all_methods(
        watched: i64,
        id: i64,
        title: &str,
        exp_watched: u32,
        exp_id: AnimeId,
    ) {
        let anime = WatchingAnime {
            watched,
            id,
            title: title.into(),
        };
        assert_eq!(anime.title(), title);
        assert_eq!(anime.watched(), exp_watched);
        assert_eq!(anime.id(), exp_id);
    }
}
