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
use crate::anilist::AnilistId;
use crate::{
    anilist::Anilist,
    anime::{AnimeId, EpisodeId, get_episode_number},
    ui::Tui,
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
    pub anilist_id: Option<AnilistId>,

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

    let (search_result, referrer) =
        utils::get_search_results(entries, watching, anilist_id, no_proxy, site).await?;

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
            let Some(name) = get_name_from_url(anime.url()) else {
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

    let anilist = Anilist::new(anilist_id)?;
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
    anime_id: AnimeId,
    episode: EpisodeId,
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

    pub fn track(&mut self, anime_id: AnimeId, episode: EpisodeId) {
        let progress = EpisodeProgress {
            anime_id,
            episode,
            percentage: 0,
            updated: false,
        };

        self.queue.push_back(progress);
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
    use simple_test_case::test_case;

    #[test_case("[status] 9%", Some(9); "single digit")]
    #[test_case("[status] 100%", Some(100); "triple digit")]
    #[test_case("[status] 09%", Some(9); "leading zero")]
    #[test_case("[status] %", None; "missing digits")]
    #[test_case("[status] no percent", None; "missing percent")]
    #[test_case("50%", Some(50); "at start of line")]
    #[test_case("AV: 00.00% (Paused)", Some(0); "paused parses digits before percent")]
    #[test_case("  75%", Some(75); "with leading spaces")]
    #[test_case("1000%", Some(1000); "four digits")]
    #[test]
    fn test_get_percentage(input: &str, expected: Option<u32>) {
        assert_eq!(get_percentage(input), expected);
    }

    #[test_case(
        "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_0017_SUB_ITA.mp4",
        Some("AnimeName");
        "standard url"
    )]
    #[test_case(
        "https://www.domain.tld/AnimeName_Ep_01.mp4",
        Some("AnimeName");
        "short url"
    )]
    #[test_case("not a url", None; "invalid url")]
    #[test_case("ftp://", None; "url without path segments")]
    #[test_case("https://www.domain.tld/", None; "root path no segments")]
    #[test_case("https://www.domain.tld/_no_name", Some(""); "underscore prefix")]
    #[test]
    fn test_get_name_from_url(url: &str, expected: Option<&str>) {
        assert_eq!(get_name_from_url(url).as_deref(), expected);
    }

    #[test_case(AnimeId(1), EpisodeId(5); "track single")]
    #[test_case(AnimeId(42), EpisodeId(1); "track another")]
    #[test]
    fn test_progress_track(anime_id: AnimeId, episode: EpisodeId) {
        let anilist = Anilist::new(None);
        let Ok(anilist) = anilist else { return };
        let mut progress = Progress::new(anilist);

        assert!(progress.queue.is_empty());

        progress.track(anime_id, episode);

        assert_eq!(progress.queue.len(), 1);
        let p = progress.queue.front().unwrap();
        assert_eq!(p.anime_id, anime_id);
        assert_eq!(p.episode, episode);
        assert_eq!(p.percentage, 0);
        assert!(!p.updated);
    }

    #[test_case(2; "track two")]
    #[test_case(3; "track three")]
    #[test]
    fn test_progress_track_multiple(count: usize) {
        let anilist = Anilist::new(None);
        let Ok(anilist) = anilist else { return };
        let mut progress = Progress::new(anilist);

        for i in 0..count {
            progress.track(AnimeId(i as u32), EpisodeId(i as u32));
        }

        assert_eq!(progress.queue.len(), count);
    }

    #[test_case(50, 75, 75; "update to higher percentage")]
    #[test_case(50, 30, 50; "ignore lower percentage")]
    #[test_case(0, 50, 50; "update from zero")]
    #[test_case(50, 50, 50; "equal percentage")]
    #[test]
    fn test_progress_update(initial: u32, new: u32, expected: u32) {
        let anilist = Anilist::new(None);
        let Ok(anilist) = anilist else { return };
        let mut progress = Progress::new(anilist);

        progress.track(AnimeId(1), EpisodeId(1));
        progress.queue.front_mut().unwrap().percentage = initial;

        progress.update(new);

        assert_eq!(progress.queue.front().unwrap().percentage, expected);
    }

    #[test_case(true, 0, true; "updated zero pops")]
    #[test_case(false, 0, false; "not updated zero stays")]
    #[test_case(true, 50, false; "updated non-zero stays")]
    #[test]
    fn test_progress_update_pop(updated: bool, percentage: u32, expected_empty: bool) {
        let anilist = Anilist::new(None);
        let Ok(anilist) = anilist else { return };
        let mut progress = Progress::new(anilist);

        progress.track(AnimeId(1), EpisodeId(1));
        progress.queue.front_mut().unwrap().updated = updated;
        progress.queue.front_mut().unwrap().percentage = percentage;

        progress.update(0);

        assert_eq!(progress.queue.is_empty(), expected_empty);
    }

    #[test_case(50; "update with empty queue")]
    #[test_case(0; "zero with empty queue")]
    #[test]
    fn test_progress_update_no_queue(percentage: u32) {
        let anilist = Anilist::new(None);
        let Ok(anilist) = anilist else { return };
        let mut progress = Progress::new(anilist);

        progress.update(percentage);
    }

    #[test_case(30, 50, 40, 50; "higher then lower")]
    #[test_case(0, 50, 30, 50; "zero then higher then lower")]
    #[test]
    fn test_progress_update_sequence(init: u32, first: u32, second: u32, expected: u32) {
        let anilist = Anilist::new(None);
        let Ok(anilist) = anilist else { return };
        let mut progress = Progress::new(anilist);

        progress.track(AnimeId(1), EpisodeId(1));
        progress.queue.front_mut().unwrap().percentage = init;

        progress.update(first);
        progress.update(second);

        assert_eq!(progress.queue.front().unwrap().percentage, expected);
    }

    #[test_case(AnimeId(0), EpisodeId(0), 0, false; "all defaults")]
    #[test_case(AnimeId(42), EpisodeId(5), 50, false; "with values")]
    #[test_case(AnimeId(99), EpisodeId(99), 100, true; "updated")]
    #[test]
    fn test_episode_progress_construction(
        anime_id: AnimeId,
        episode: EpisodeId,
        percentage: u32,
        updated: bool,
    ) {
        let p = EpisodeProgress {
            anime_id,
            episode,
            percentage,
            updated,
        };
        assert_eq!(p.anime_id, anime_id);
        assert_eq!(p.episode, episode);
        assert_eq!(p.percentage, percentage);
        assert_eq!(p.updated, updated);
    }

    #[test_case(0, true; "default is empty")]
    #[test]
    fn test_progress_default(_dummy: u32, expected_empty: bool) {
        let progress = Progress::default();
        assert_eq!(progress.queue.is_empty(), expected_empty);
    }
}
