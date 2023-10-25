use graphql_client::{GraphQLQuery, Response};
use reqwest::{header, header::HeaderValue, Client};

use crate::config::{load_config, save_config};
use crate::tui;

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

pub async fn last_watched(client_id: Option<u32>, anime_id: Option<u32>) -> Option<u32> {
    let client = new_client(client_id)?;

    let url = "https://graphql.anilist.co";
    let variables = progress_query::Variables {
        id: anime_id.map(|u| u as i64),
    };
    let query = ProgressQuery::build_query(variables);
    let res = client.post(url).json(&query).send().await.ok()?;
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

pub async fn get_watching_list(client_id: Option<u32>) -> Option<Vec<(String, i64)>> {
    let client = new_client(client_id)?;

    let url = "https://graphql.anilist.co";
    let query = UserQuery::build_query(user_query::Variables);
    let res = client.post(url).json(&query).send().await.ok()?;
    let response_body = res
        .json::<Response<user_query::ResponseData>>()
        .await
        .ok()?;
    let user_id = response_body.data?.viewer.map(|d| d.id);

    let variables = watching_query::Variables { id: user_id };
    let query = WatchingQuery::build_query(variables);

    let res = client.post(url).json(&query).send().await.ok()?;
    let response_body = res
        .json::<Response<watching_query::ResponseData>>()
        .await
        .ok()?;

    let mut list = response_body.data?.media_list_collection?.lists?[0]
        .clone()?
        .entries?
        .into_iter()
        .filter_map(|e| {
            e.and_then(|m| m.media)
                .and_then(|m| m.title.zip(Some(m.id)))
                .and_then(|(t, id)| t.romaji.zip(Some(id)))
        })
        .collect::<Vec<_>>();
    list.sort_unstable_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

    Some(list)
}

fn new_client(client_id: Option<u32>) -> Option<Client> {
    let client_id = client_id.unwrap_or(4047);
    let token = load_config().map_or_else(|_| oauth_token(client_id), Some)?;

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

    Client::builder().default_headers(headers).build().ok()
}

fn oauth_token(client_id: u32) -> Option<String> {
    let url = format!(
        "https://anilist.co/api/v2/oauth/authorize?response_type=token&client_id={client_id}"
    );
    let token = tui::get_token(&url).ok()?;
    save_config(&token).ok()?;

    Some(token)
}
