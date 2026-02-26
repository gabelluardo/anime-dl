use clap::Parser;

use std::path::PathBuf;
use std::sync::Arc;

use super::Site;
use crate::anime::get_episode_number;
use crate::archive::AnimeWorld;
use crate::range::Range;
use crate::scraper::ProxyManager;
use crate::tui::Tui;

use anyhow::{Result, anyhow, ensure};
use futures::stream::StreamExt;
use reqwest::Client;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use tokio::{fs, io::AsyncWriteExt};
use tokio_stream as stream;

/// Download anime
#[derive(Parser, Debug, Default)]
#[command(arg_required_else_help(true))]
pub struct Args {
    /// Source urls or scraper's queries
    pub entries: Vec<String>,

    /// Save files in a folder with a default name
    #[arg(short = 'D', long = "default-dir")]
    pub auto_dir: bool,

    /// Maximum number of simultaneous downloads allowed
    #[arg(
        default_value = "24",
        short = 'm',
        long = "max-concurrent",
        name = "MAX"
    )]
    pub dim_buff: usize,

    /// Root path where store files
    #[arg(default_value = ".", short, long)]
    pub dir: PathBuf,

    /// Override existent files
    #[arg(short, long)]
    pub force: bool,

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
    let client_id = args.anilist_id;
    let site = args.site.unwrap_or_default();

    let searches = if args.watching || args.entries.is_empty() {
        super::get_from_watching_list(client_id).await?
    } else {
        super::get_from_input(args.entries)?
    };

    let proxy = ProxyManager::proxy(args.no_proxy).await;

    let (vec_anime, referrer) = match site {
        Site::AW => super::search_site::<AnimeWorld>(&searches, proxy).await?,
    };

    let ui = Tui::new();
    let client = Arc::new(Client::new());
    let mut pool = vec![];
    for anime in &vec_anime {
        let episodes: Vec<String> = match args.range {
            Some(range) if !args.interactive => anime.select_from_range(range),
            _ => Tui::select_episodes(anime)?,
        };

        let mut parent = args.dir.clone();
        if args.auto_dir {
            let name = get_dir_name(&anime.url)?;
            let dir = camel_to_snake(&name);

            parent.push(dir);
        }

        for url in episodes {
            let pb = ui.add_bar();
            let mut path = parent.clone();
            let client = client.clone();

            let future = async move {
                let filename = get_filename(&url)?;
                let source_size = client
                    .head(&url)
                    .header(REFERER, referrer)
                    .send()
                    .await?
                    .error_for_status()?
                    .headers()
                    .get(CONTENT_LENGTH)
                    .and_then(|ct_len| ct_len.to_str().ok())
                    .and_then(|ct_len| ct_len.parse().ok())
                    .unwrap_or_default();

                let mut dest = {
                    if !path.exists() {
                        fs::create_dir_all(&path).await?;
                    }
                    path.push(&filename);

                    fs::OpenOptions::new()
                        .append(!args.force)
                        .truncate(args.force)
                        .write(args.force)
                        .create(true)
                        .open(path)
                        .await?
                };

                let file_size = dest.metadata().await?.len();
                ensure!(file_size < source_size, filename + " already exists");

                let msg = match get_episode_number(&url) {
                    Some(num) => format!(
                        "Ep. {:0fill$} {}",
                        num.value,
                        anime.name,
                        fill = num.alignment
                    ),
                    _ => anime.name.clone(),
                };

                pb.set_position(file_size);
                pb.set_length(source_size);
                pb.set_message(msg);

                let mut source = client
                    .get(&url)
                    .header(RANGE, format!("bytes={file_size}-"))
                    .header(REFERER, referrer)
                    .send()
                    .await?
                    .error_for_status()?;
                while let Some(chunk) = source.chunk().await? {
                    dest.write_all(&chunk).await?;
                    pb.inc(chunk.len() as u64);
                }

                pb.finish_with_message(pb.message() + " 👍");

                Ok(())
            };

            pool.push(future);
        }
    }

    stream::iter(pool)
        .buffer_unordered(args.dim_buff.max(1))
        .collect::<Vec<_>>()
        .await;

    Ok(())
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
