use anyhow::{Result, anyhow};

use graphql_client::{GraphQLQuery, Response};
use reqwest::{Client, header, header::HeaderValue};

use crate::config::{load_config, save_config};
use crate::tui::Tui;

const ENDPOINT: &str = "https://graphql.anilist.co";

#[derive(Debug)]
pub struct WatchingAnime {
    watched: i64,
    id: i64,
    title: String,
}

impl WatchingAnime {
    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn watched(&self) -> u32 {
        self.watched as u32
    }

    pub fn id(&self) -> u32 {
        self.id as u32
    }
}

#[derive(GraphQLQuery, Debug)]
#[graphql(
    schema_path = "schema/anilist_schema.json",
    query_path = "schema/progress_query.graphql"
)]
struct ProgressQuery;

impl ProgressQuery {
    async fn get(client: &Client, id: u32) -> Option<(i64, i64)> {
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

        Some((episodes, progress))
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
        id: u32,
        progress: u32,
        status: progress_mutation::MediaListStatus,
    ) -> Result<()> {
        let variables = progress_mutation::Variables {
            status: Some(status),
            id: Some(id.into()),
            progress: Some(progress as i64),
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
    query_path = "schema/watching_query.graphql",
    response_derives = "Clone, Default"
)]
struct WatchingQuery;

impl WatchingQuery {
    async fn get(client: &Client, id: i64) -> Option<Vec<WatchingAnime>> {
        let id = Some(id);
        let variables = watching_query::Variables { id };
        let query = WatchingQuery::build_query(variables);
        let response = client.post(ENDPOINT).json(&query).send().await.ok()?;
        let json = response
            .json::<Response<watching_query::ResponseData>>()
            .await
            .ok()?;

        let mut list: Vec<_> = json.data?.media_list_collection?.lists?[0]
            .take()?
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

        list.sort_unstable_by(|a, b| a.title.partial_cmp(&b.title).unwrap());

        Some(list)
    }
}

#[derive(GraphQLQuery, Debug)]
#[graphql(
    schema_path = "schema/anilist_schema.json",
    query_path = "schema/user_query.graphql"
)]
struct UserQuery;

impl UserQuery {
    async fn get(client: &Client) -> Option<i64> {
        let query = UserQuery::build_query(user_query::Variables);
        let response = client.post(ENDPOINT).json(&query).send().await.ok()?;
        let json = response
            .json::<Response<user_query::ResponseData>>()
            .await
            .ok()?;

        json.data?.viewer.map(|d| d.id)
    }
}

#[derive(Default, Debug)]
pub struct Anilist {
    client: Client,
}

impl Anilist {
    pub fn new(client_id: Option<u32>) -> Result<Self> {
        let client_id = client_id.unwrap_or(4047);
        let token = load_config("anilist", "token").map_or_else(|_| oauth_token(client_id), Ok)?;

        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
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

    pub async fn get_progress(&self, id: u32) -> Option<(i64, i64)> {
        let (episodes, progress) = ProgressQuery::get(&self.client, id).await?;

        Some((episodes, progress))
    }

    pub async fn update(&mut self, id: u32, number: u32) -> Result<()> {
        use progress_mutation::MediaListStatus;

        let progress = self.get_progress(id).await;
        let status = match progress {
            Some((_, last)) if last >= number.into() => return Ok(()),
            Some((ep, _)) if ep <= number.into() => MediaListStatus::COMPLETED,
            None | Some((_, _)) => MediaListStatus::CURRENT,
        };

        ProgressMutation::put(&self.client, id, number, status).await?;

        Ok(())
    }
}

fn oauth_token(client_id: u32) -> Result<String> {
    let url = format!(
        "https://anilist.co/api/v2/oauth/authorize?response_type=token&client_id={client_id}"
    );
    let token = Tui::get_token(&url);

    save_config("anilist", "token", &token)?;

    Ok(token)
}
