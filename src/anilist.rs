use anyhow::{Result, anyhow};

use graphql_client::{GraphQLQuery, Response};
use reqwest::{Client, header, header::HeaderValue};

use crate::config::{load_config, save_config};
use crate::tui;

const ENDPOINT: &str = "https://graphql.anilist.co";

#[derive(Clone, Debug)]
pub struct WatchingAnime {
    pub behind: u32,
    pub id: u32,
    pub title: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist_schema.graphql",
    query_path = "graphql/progress_query.graphql"
)]
struct ProgressQuery;

#[derive(GraphQLQuery, Debug)]
#[graphql(
    schema_path = "graphql/anilist_schema.graphql",
    query_path = "graphql/progress_mutation.graphql"
)]
struct ProgressMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist_schema.graphql",
    query_path = "graphql/watching_query.graphql",
    response_derives = "Clone, Default"
)]
struct WatchingQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/anilist_schema.graphql",
    query_path = "graphql/user_query.graphql"
)]
struct UserQuery;

pub async fn get_watching_list(client_id: Option<u32>) -> Option<Vec<WatchingAnime>> {
    let client = new_client(client_id).ok()?;

    let query = UserQuery::build_query(user_query::Variables);
    let res = client.post(ENDPOINT).json(&query).send().await.ok()?;
    let response_body = res
        .json::<Response<user_query::ResponseData>>()
        .await
        .ok()?;
    let user_id = response_body.data?.viewer.map(|d| d.id);

    let variables = watching_query::Variables { id: user_id };
    let query = WatchingQuery::build_query(variables);

    let res = client.post(ENDPOINT).json(&query).send().await.ok()?;
    let response_body = res
        .json::<Response<watching_query::ResponseData>>()
        .await
        .ok()?;

    let mut list = response_body.data?.media_list_collection?.lists?[0]
        .clone()?
        .entries?
        .into_iter()
        .filter_map(|e| {
            let progress = e
                .as_ref()
                .and_then(|c| c.progress)
                .map(|p| p as u32)
                .unwrap_or_default();

            e.and_then(|e| e.media)
                .and_then(|m| {
                    m.title.zip(Some(m.id)).zip(
                        m.episodes
                            .zip(Some(m.next_airing_episode.unwrap_or_default())),
                    )
                })
                .and_then(|((t, id), (e, n))| {
                    t.romaji
                        .zip(Some(id as u32))
                        .zip(Some((e as u32, n.episode as u32)))
                })
                .map(|((t, id), (e, n))| WatchingAnime {
                    id,
                    title: t,
                    behind: n.checked_sub(progress + 1).unwrap_or(e - progress),
                })
        })
        .collect::<Vec<_>>();

    list.sort_unstable_by(|a, b| a.title.partial_cmp(&b.title).unwrap());

    Some(list)
}

pub async fn last_watched(client_id: Option<u32>, anime_id: Option<u32>) -> Option<u32> {
    let client = new_client(client_id).ok()?;

    let variables = progress_query::Variables {
        id: anime_id.map(|u| u as i64),
    };
    let query = ProgressQuery::build_query(variables);
    let res = client.post(ENDPOINT).json(&query).send().await.ok()?;
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

pub async fn update_watched(
    client_id: Option<u32>,
    anime_id: Option<u32>,
    number: u32,
) -> Result<()> {
    if last_watched(client_id, anime_id).await > Some(number) {
        return Ok(());
    }

    let client = new_client(client_id)?;

    let variables = progress_mutation::Variables {
        id: anime_id.map(|u| u as i64),
        progress: Some(number as i64),
    };
    let query = ProgressMutation::build_query(variables);
    client
        .post(ENDPOINT)
        .json(&query)
        .send()
        .await?
        .error_for_status()?;

    // println!("Updating episode {} for anime {:?}", number, anime_id);

    Ok(())
}

fn new_client(client_id: Option<u32>) -> Result<Client> {
    let client_id = client_id.unwrap_or(4047);
    let token = load_config().map_or_else(|_| oauth_token(client_id), Ok)?;

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

    Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|_| anyhow!("Unable to create client"))
}

fn oauth_token(client_id: u32) -> Result<String> {
    let url = format!(
        "https://anilist.co/api/v2/oauth/authorize?response_type=token&client_id={client_id}"
    );
    let token = tui::get_token(&url)?;
    save_config(&token)?;

    Ok(token)
}
