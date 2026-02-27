use clap::Parser;

use std::path::PathBuf;

use super::Site;
use crate::anime::get_episode_number;
use crate::archive::AnimeWorld;
use crate::range::Range;
use crate::scraper::ProxyManager;
use crate::tui::Tui;

use anyhow::{Context, Result, anyhow};
use futures::stream::StreamExt;
use reqwest::header::{CONTENT_LENGTH, REFERER};
use reqwest::{Client, Response};
use tokio::{fs, io::AsyncWriteExt};
use tokio_stream as stream;

/// Download anime
#[derive(Parser, Debug, Default)]
#[command(arg_required_else_help(true))]
pub struct Args {
    /// Source urls or scraper's queries
    pub entries: Vec<String>,

    /// Maximum number of simultaneous downloads allowed
    #[arg(
        default_value = "24",
        short = 'm',
        long = "max-concurrent",
        name = "MAX"
    )]
    pub max_concurrent: usize,

    /// Root path where store files
    #[arg(default_value = ".", short, long)]
    pub destination: PathBuf,

    /// Interactive mode
    #[arg(short, long, conflicts_with = "range")]
    pub interactive: bool,

    /// Episodes to download (es. `1-4` or `1,2,3,4`) [default: 1]
    #[arg(short = 'r', long = "range")]
    pub range: Option<Range<u32>>,

    /* Common parameters */
    /// Override app id environment variable    
    #[arg(short, long, env = "ANIMEDL_ID", hide_env_values = true)]
    pub anilist_id: Option<u32>,

    /// Disable automatic proxy (useful for slow connections)
    #[arg(short = 'p', long)]
    pub no_proxy: bool,

    /// Search anime in remote archive
    #[arg(long, short = 'S', value_enum)]
    pub site: Option<Site>,

    /// Retrieve anime from anilist watching list        
    #[arg(short, long)]
    pub watching: bool,
}

pub async fn exec(args: Args) -> Result<()> {
    let Args {
        entries,
        max_concurrent,
        destination,
        interactive,
        range,
        anilist_id,
        no_proxy,
        site,
        watching,
    } = args;

    let searches = if watching || entries.is_empty() {
        super::get_from_watching_list(anilist_id).await?
    } else {
        super::get_from_input(entries)?
    };

    let proxy = ProxyManager::proxy(no_proxy).await;

    let (search_result, referrer) = match site {
        Some(Site::AW) | None => super::search_site::<AnimeWorld>(&searches, proxy).await?,
    };

    let ui = Tui::new();
    let client = Client::new();

    let mut pool = Vec::new();
    for anime in &search_result {
        let episodes: Vec<String> = match range {
            Some(range) if !interactive => anime.select_from_range(range),
            _ => Tui::select_episodes(anime)?,
        };

        let root = {
            let mut root = destination.clone();
            let name = get_dir_name(&anime.url)?;
            let dir = camel_to_snake(&name);
            root.push(dir);

            root
        };
        fs::create_dir_all(&root).await?;

        for url in episodes {
            let pb = ui.add_bar();
            let client = client.clone();
            let name = anime.name.clone();
            let dest = {
                let mut dest = root.clone();
                let filename = get_filename(&url)?;
                dest.push(filename);

                dest
            };
            let tmp_dest = {
                let mut tmp_dest = dest.clone();
                tmp_dest.add_extension("tmp");

                tmp_dest
            };

            let future = async move {
                let source_size = get_source_size(&client, &url, referrer).await?;
                let msg = get_progress_message(&url, &name);
                pb.set_position(0);
                pb.set_length(source_size);
                pb.set_message(msg);

                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&tmp_dest)
                    .await?;

                let mut source = get(&client, &url, referrer).await?;
                while let Some(chunk) = source.chunk().await? {
                    file.write_all(&chunk).await?;
                    pb.inc(chunk.len() as u64);
                }

                fs::copy(&tmp_dest, &dest).await?;
                fs::remove_file(tmp_dest).await?;

                pb.finish_with_message(pb.message() + " 👍");

                anyhow::Ok(())
            };

            pool.push(future);
        }
    }

    stream::iter(pool)
        .buffer_unordered(max_concurrent.max(1))
        .collect::<Vec<_>>()
        .await;

    Ok(())
}

async fn get_source_size(client: &Client, url: &str, referrer: &str) -> Result<u64> {
    let response = client
        .head(url)
        .header(REFERER, referrer)
        .send()
        .await?
        .error_for_status()?;
    let content_len = response
        .headers()
        .get(CONTENT_LENGTH)
        .context("unable to get content length")?;

    let size = content_len.to_str()?.parse()?;

    Ok(size)
}
async fn get(client: &Client, url: &str, referrer: &str) -> Result<Response> {
    let response = client
        .get(url)
        .header(REFERER, referrer)
        .send()
        .await?
        .error_for_status()?;

    Ok(response)
}

fn get_progress_message(url: &str, name: &str) -> String {
    match get_episode_number(url) {
        Some(num) => format!("Ep. {:0fill$} {}", num.value, name, fill = num.alignment),
        _ => name.to_owned(),
    }
}

/// Extract the filename from a media URL.
fn get_filename(input: &str) -> Result<String> {
    reqwest::Url::parse(input)?
        .path_segments()
        .and_then(|mut s| s.next_back())
        .map(|s| s.into())
        .ok_or(anyhow!("Unable to get {input}"))
}

/// Extract the directory name from a media URL (before the first underscore).
fn get_dir_name(input: &str) -> Result<String> {
    let url = reqwest::Url::parse(input)?;
    url.path_segments()
        .and_then(|mut s| s.next_back())
        .and_then(|s| s.split('_').next())
        .map(|s| s.into())
        .ok_or(anyhow!("Unable to get {input}"))
}

/// Convert a camelCase/PascalCase string into snake_case.
pub fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev = None;

    for c in s.chars() {
        if c.is_ascii_uppercase()
            && prev.is_some_and(|p: char| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }

        out.push(c.to_ascii_lowercase());
        prev = Some(c);
    }

    out
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_filename() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let res = get_filename(url).unwrap();
        assert_eq!(res, "AnimeName_Ep_15_SUB_ITA.mp4")
    }

    #[test]
    fn test_get_dir_name() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let res = get_dir_name(url).unwrap();
        assert_eq!(res, "AnimeName")
    }

    #[test]
    fn test_camel_to_snake() {
        let res = camel_to_snake("AnimeName");
        assert_eq!(res, "anime_name");

        let res = camel_to_snake("IDInvaded");
        assert_eq!(res, "idinvaded");

        let res = camel_to_snake("SwordArtOnline2");
        assert_eq!(res, "sword_art_online2");

        let res = camel_to_snake("SlimeTaoshite300-nen");
        assert_eq!(res, "slime_taoshite300-nen")
    }
}
