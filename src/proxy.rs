use anyhow::Result;
use rand::seq::IteratorRandom;

pub struct ProxyManager;

impl ProxyManager {
    pub async fn proxy(disable: bool) -> Option<String> {
        if disable {
            return None;
        }

        Self::get_random_proxy().await.ok()
    }

    async fn get_random_proxy() -> Result<String> {
        let url = "https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=2000&country=all&ssl=all&anonymity=elite";

        let list = reqwest::get(url).await?.text().await?;
        let Some(chosen) = list.split_ascii_whitespace().choose(&mut rand::rng()) else {
            return Err(anyhow::anyhow!("No proxy found"));
        };

        let proxy = format!("https://{chosen}");

        Ok(proxy)
    }
}
