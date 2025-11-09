use clap::Parser;

use std::collections::HashMap;
use std::process::Stdio;

use anyhow::{Result, bail};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::LinesStream;
use which::which;

use super::{Progress, Site};
use crate::anilist::Anilist;
use crate::archive::{AnimeWorld, Archive};
use crate::parser::{parse_number, parse_percentage, parse_url};
use crate::scraper::{CookieManager, ProxyManager, Scraper, ScraperConfig};
use crate::tui::Tui;

/// Stream anime in a media player
#[derive(Parser, Debug)]
#[command(arg_required_else_help(true))]
pub struct Args {
    /// Source urls or scraper's queries
    pub entries: Vec<String>,

    /*  Common paramenters */
    /// Override app id environment variable    
    #[cfg(feature = "anilist")]
    #[arg(short, long, env = "ANIMEDL_ID", hide_env_values = true)]
    pub anilist_id: Option<u32>,

    /// Disable automatic proxy (useful for slow connections)
    #[arg(short = 'p', long)]
    pub no_proxy: bool,

    /// Search anime in remote archive
    #[arg(long, short = 'S', value_enum)]
    pub site: Option<Site>,

    /// Retrive anime from anilist watching list        
    #[cfg(feature = "anilist")]
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
        Site::AW => {
            let cookie = CookieManager::extract_cookie_for_site::<AnimeWorld>().await;
            let config = ScraperConfig { proxy, cookie };

            (
                Scraper::new(config).search::<AnimeWorld>(&searches).await?,
                AnimeWorld::REFERRER,
            )
        }
    };

    let (cmd, cmd_referrer) = if let Ok(c) = which("mpv") {
        (c, format!("--referrer={referrer}"))
    } else if let Ok(c) = which("vlc") {
        (c, format!("--http-referrer={referrer}"))
    } else {
        bail!("No supported media player found. Please install mpv or vlc.")
    };

    let mut episodes = vec![];
    let mut ids = HashMap::new();
    for anime in &vec_anime {
        ids.insert(&anime.url, anime.id);
        episodes.extend(Tui::select_episodes(anime)?);
    }

    let mut child = Command::new(cmd)
        .arg(&cmd_referrer)
        .arg("-v")
        .args(&episodes)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    {
        let stdout = BufReader::new(child.stdout.as_mut().unwrap());
        let stderr = BufReader::new(child.stderr.as_mut().unwrap());
        let stdout_lines = LinesStream::new(stdout.lines());
        let stderr_lines = LinesStream::new(stderr.lines());

        let mut merged = tokio_stream::StreamExt::merge(stdout_lines, stderr_lines);

        let mut progress = Progress::new(Anilist::new(client_id)?);
        while let Some(Ok(line)) = merged.next().await {
            match line.as_str() {
                line if line.contains("Opening done") => {
                    if let Some(url) = line.split_whitespace().last() {
                        let num = parse_number(url);
                        let origin = parse_url(url, num);

                        let anime_id = ids.get(&origin).copied().flatten();
                        let episode = num.map(|n| n.value);

                        progress.push(anime_id, episode);
                    }
                }

                line if line.contains('%') && !line.contains("(Paused)") => {
                    progress.percentage(parse_percentage(line));
                }

                _ => {}
            }

            progress.update().await;
        }
    }

    Ok(())
}
