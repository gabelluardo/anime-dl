use anyhow::Result;
use reqwest::{header, header::HeaderValue, Client};
use tokio::{fs, io::AsyncWriteExt, task};

pub struct AniListBuilder {
    client_id: String,
    token: Option<String>,
}

impl<'a> AniListBuilder {
    const AUTHORIZATION: &'a str = "Bearer ";
    const ACCEPT: &'a str = "application/json";
    const OAUTH_URL: &'a str = "https://anilist.co/api/v2/oauth/authorize?\
        response_type=token&client_id=";

    pub fn build(self) -> Result<AniList> {
        let oauth_url = format!("{}{}", Self::OAUTH_URL, self.client_id);

        let token = match self.load_conf() {
            Some(t) => t,
            _ => {
                println!(
                    "For autentication go to: {}\n\nAnd paste token here:",
                    oauth_url
                );
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;

                line.trim().to_string()
            }
        };

        let mut headers = header::HeaderMap::new();
        let auth = HeaderValue::from_str(&format!("{}{}", Self::AUTHORIZATION, token))?;

        headers.insert(header::AUTHORIZATION, auth);
        headers.insert(header::ACCEPT, HeaderValue::from_static(Self::ACCEPT));
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(Self::ACCEPT));

        let client = Client::builder().default_headers(headers).build()?;

        // js
        // var url = 'https://graphql.anilist.co',
        //     options = {
        //         method: 'POST',
        //         headers: {
        //             'Authorization': 'Bearer ' + accessToken,
        //             'Content-Type': 'application/json',
        //             'Accept': 'application/json',
        //         },
        //         body: JSON.stringify({
        //             query: query
        //         })
        //     };
        // ​
        // fetch(url, options).then(handleResponse, handleError);
        // ​
        // function handleResponse (response) {
        //     console.log(response);
        // }

        Ok(AniList {
            client,
            anime_id: None,
        })
    }

    pub fn token(self, token: Option<String>) -> Self {
        Self { token, ..self }
    }

    pub fn env_client_id(self) -> Self {
        dotenv::dotenv().ok();

        Self {
            client_id: dotenv::var("CLIENT_ID").unwrap_or(self.client_id),
            ..self
        }
    }

    #[cfg(not(windows))]
    fn load_conf(&self) -> Option<String> {
        // controlla self.token e poi il file salvato
        None
    }

    #[cfg(windows)]
    fn load_conf(&self) -> Option<String> {
        None
    }
}
impl Default for AniListBuilder {
    fn default() -> Self {
        Self {
            client_id: String::from("4047"),
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

    pub fn new() -> Self {
        AniListBuilder::default().build().expect("AniList::new()")
    }

    pub fn builder() -> AniListBuilder {
        AniListBuilder::default()
    }

    pub fn id(self, id: u32) -> Self {
        Self {
            anime_id: Some(id),
            ..self
        }
    }

    pub fn get(&self, query: String) {}

    pub fn last_viewed(&self) -> Option<u32> {
        None
    }
}
