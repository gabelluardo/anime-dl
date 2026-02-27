use clap::Parser;

use std::collections::HashMap;
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::LinesStream;
use which::which;

use super::{Progress, Site};
use crate::anilist::Anilist;
use crate::anime::{get_episode_number, remove_episode_number};
use crate::archive::AnimeWorld;
use crate::scraper::ProxyManager;
use crate::tui::Tui;

/// Stream anime in a media player
#[derive(Parser, Debug)]
#[command(arg_required_else_help(true))]
pub struct Args {
    /// Source urls or scraper's queries
    pub entries: Vec<String>,

    /*  Common parameters */
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

    let (cmd, cmd_referrer) = if let Ok(c) = which("mpv") {
        (c, format!("--referrer={referrer}"))
    } else if let Ok(c) = which("vlc") {
        (c, format!("--http-referrer={referrer}"))
    } else {
        bail!("No supported media player found. Please install mpv or vlc.")
    };

    let (ids, episodes) = {
        let mut ids = HashMap::new();
        let mut episodes = Vec::new();
        for anime in &search_result {
            ids.insert(&anime.url, anime.id);
            episodes.extend(Tui::select_episodes(anime)?);
        }

        (ids, episodes)
    };

    let child = Command::new(cmd)
        .arg(&cmd_referrer)
        .arg("-v")
        .args(&episodes)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let mut stream = {
        let stdout = child
            .stdout
            .map(BufReader::new)
            .context("Unable to get stdout")?;
        let stderr = child
            .stderr
            .map(BufReader::new)
            .context("Unable to get stderr")?;
        let stdout_lines = LinesStream::new(stdout.lines());
        let stderr_lines = LinesStream::new(stderr.lines());

        StreamExt::merge(stdout_lines, stderr_lines)
    };

    let mut progress = Progress::new(Anilist::new(anilist_id)?);
    while let Some(Ok(line)) = stream.next().await {
        match line {
            line if line.contains("Opening done") => {
                if let Some(url) = line.split_whitespace().last() {
                    let num = get_episode_number(url);
                    let origin = remove_episode_number(url, num);

                    let anime_id = ids.get(&origin).copied().flatten();
                    let episode = num.map(|n| n.value);

                    progress.track(anime_id, episode);
                }
            }

            line if line.contains('%') && !line.contains("(Paused)") => {
                progress.update(get_percentage(&line));
                progress.send().await
            }

            _ => {}
        }
    }

    Ok(())
}

/// Extract the percentage value from a player output line.
fn get_percentage(input: &str) -> Option<u32> {
    let sym = input.find('%')?;

    // SAFE: `input` is a simple ASCII string
    let bytes = input.as_bytes();

    let mut start = sym;
    while start > 0 && bytes[start - 1].is_ascii_digit() {
        start -= 1;
    }

    if start == sym {
        return None;
    }

    input.get(start..sym)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::get_percentage;

    #[test]
    fn test_get_percentage() {
        assert_eq!(get_percentage("[status] 9%"), Some(9));
        assert_eq!(get_percentage("[status] 100%"), Some(100));
        assert_eq!(get_percentage("[status] 09%"), Some(9));
        assert_eq!(get_percentage("[status] %"), None);
        assert_eq!(get_percentage("[status] no percent"), None);
    }
}
