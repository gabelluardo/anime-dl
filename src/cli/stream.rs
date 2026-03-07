use std::collections::{HashMap, VecDeque};
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use clap::Parser;
use reqwest::Url;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::LinesStream;
use which::which;

use super::{Site, utils};
use crate::{
    anilist::Anilist, anime::get_episode_number, archives::AnimeWorld, proxy::ProxyManager, ui::Tui,
};

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

    let anilist = Anilist::new(anilist_id)?;
    let searches = if watching || entries.is_empty() {
        utils::get_from_watching_list(&anilist).await?
    } else {
        utils::get_from_input(entries)?
    };

    let proxy = ProxyManager::proxy(no_proxy).await;
    let (search_result, referrer) = match site {
        Some(Site::AW) | None => utils::search_site::<AnimeWorld>(&searches, proxy).await?,
    };

    let (cmd, cmd_referrer) = if let Ok(c) = which("mpv") {
        (c, format!("--referrer={referrer}"))
    } else if let Ok(c) = which("vlc") {
        (c, format!("--http-referrer={referrer}"))
    } else {
        bail!("no supported media player found. Please install mpv or vlc.")
    };

    let (ids, episodes) = {
        let mut ids = HashMap::new();
        let mut episodes = Vec::new();
        for anime in &search_result {
            let Some(name) = get_name_from_url(&anime.url()) else {
                continue;
            };

            ids.insert(name, anime.id());
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

    let mut progress = Progress::new(anilist);
    while let Some(Ok(line)) = stream.next().await {
        match line {
            line if line.contains("Opening done") => {
                let Some(url) = line.split_whitespace().last() else {
                    continue;
                };
                let Some((num, _)) = get_episode_number(url) else {
                    continue;
                };
                let Some(name) = get_name_from_url(url) else {
                    continue;
                };
                let Some(id) = ids.get(&name).copied().flatten() else {
                    continue;
                };

                progress.track(id, num);
            }

            line if line.contains('%') && !line.contains("(Paused)") => {
                let Some(percentage) = get_percentage(&line) else {
                    continue;
                };

                progress.update(percentage);
                progress.send().await
            }

            _ => {}
        }
    }

    Ok(())
}

// Get anime name parsing anime url.
fn get_name_from_url(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    let last = url.path_segments()?.next_back()?;
    let name = last.split_terminator("_").next()?.to_string();

    Some(name)
}

/// Extract the percentage value from a player output line.
fn get_percentage(line: &str) -> Option<u32> {
    let sym = line.find('%')?;
    let bytes = line.as_bytes();

    let mut start = sym;
    while start > 0 && bytes[start - 1].is_ascii_digit() {
        start -= 1;
    }

    if start == sym {
        return None;
    }

    line.get(start..sym)?.parse().ok()
}

#[derive(Default, Debug)]
struct EpisodeProgress {
    anime_id: u32,
    episode: u32,
    percentage: u32,
    updated: bool,
}

#[derive(Default, Debug)]
struct Progress {
    anilist: Anilist,
    queue: VecDeque<EpisodeProgress>,
}

impl Progress {
    const MIN_PERCENTAGE: u32 = 80;

    pub fn new(anilist: Anilist) -> Self {
        let queue = VecDeque::new();

        Self { anilist, queue }
    }

    pub fn track(&mut self, anime_id: u32, episode: u32) {
        let progess = EpisodeProgress {
            anime_id,
            episode,
            percentage: 0,
            updated: false,
        };

        self.queue.push_back(progess);
    }

    pub fn update(&mut self, percentage: u32) {
        let Self { queue, .. } = self;

        match queue.front_mut() {
            // update current episode
            Some(p) if p.percentage <= percentage => {
                p.percentage = percentage;
            }

            // new episode is selected, pass to the next
            Some(EpisodeProgress { updated: true, .. }) if percentage == 0 => {
                queue.pop_front();
            }

            _ => (),
        }
    }

    pub async fn send(&mut self) {
        let Self { anilist, queue } = self;

        if let Some(p) = queue.front_mut()
            && !p.updated
            && p.percentage > Progress::MIN_PERCENTAGE
        {
            let result = anilist.update(p.anime_id, p.episode).await;
            p.updated = result.is_ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_percentage() {
        assert_eq!(get_percentage("[status] 9%"), Some(9));
        assert_eq!(get_percentage("[status] 100%"), Some(100));
        assert_eq!(get_percentage("[status] 09%"), Some(9));
        assert_eq!(get_percentage("[status] %"), None);
        assert_eq!(get_percentage("[status] no percent"), None);
    }

    #[test]
    fn test_get_name_from_url() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_0017_SUB_ITA.mp4";
        assert_eq!(get_name_from_url(url).unwrap(), "AnimeName")
    }
}
