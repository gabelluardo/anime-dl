use anyhow::Result;
use rand::seq::IteratorRandom;
use reqwest::{Client, Url};

pub struct ProxyConfig {
    base_url: &'static str,
    request: &'static str,
    proxy_type: &'static str,
    timeout: u32,
    country: &'static str,
    ssl: &'static str,
    anonymity: &'static str,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.proxyscrape.com/",
            request: "getproxies",
            proxy_type: "http",
            timeout: 2000,
            country: "all",
            ssl: "all",
            anonymity: "elite",
        }
    }
}

impl ProxyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_url(&self) -> Result<Url> {
        let mut url = Url::parse(self.base_url)?;
        url.query_pairs_mut()
            .append_pair("request", self.request)
            .append_pair("proxytype", self.proxy_type)
            .append_pair("timeout", &self.timeout.to_string())
            .append_pair("country", self.country)
            .append_pair("ssl", self.ssl)
            .append_pair("anonymity", self.anonymity);
        Ok(url)
    }
}

pub async fn get_random_proxy(client: &Client, config: ProxyConfig) -> Result<String> {
    let url = config.to_url()?;

    let list = client.get(url).send().await?.text().await?;
    let Some(chosen) = list.split_ascii_whitespace().choose(&mut rand::rng()) else {
        return Err(anyhow::anyhow!("No proxy found"));
    };

    let proxy = format!("https://{chosen}");

    Ok(proxy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_defaults() {
        let config = ProxyConfig::new();
        let url = config.to_url().unwrap();
        let url_str = url.to_string();

        assert!(url_str.contains("https://api.proxyscrape.com/"));
        assert!(url_str.contains("request=getproxies"));
        assert!(url_str.contains("proxytype=http"));
        assert!(url_str.contains("timeout=2000"));
        assert!(url_str.contains("country=all"));
        assert!(url_str.contains("ssl=all"));
        assert!(url_str.contains("anonymity=elite"));
    }

    #[test]
    fn test_proxy_config_to_url_format() {
        let config = ProxyConfig::new();
        let url = config.to_url().unwrap();
        let url_str = url.to_string();

        // Verify all query parameters are present
        assert!(url_str.contains("request="));
        assert!(url_str.contains("proxytype="));
        assert!(url_str.contains("timeout="));
        assert!(url_str.contains("country="));
        assert!(url_str.contains("ssl="));
        assert!(url_str.contains("anonymity="));
    }
}
