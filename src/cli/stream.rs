use clap::Parser;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::LinesStream;
use which::which;

use super::{Progress, Site};
use crate::anilist::Anilist;
use crate::parser::{parse_number, parse_percentage, parse_url};
use crate::scraper::{Scraper, find_cookie, select_proxy};
use crate::tui;

/// Stream anime in a media player
#[derive(Parser, Debug)]
#[command(arg_required_else_help(true))]
pub struct Args {
    /// Source urls or scraper's queries
    #[arg(required_unless_present("watching"))]
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

pub async fn execute(cmd: Args) -> Result<()> {
    let client_id = cmd.anilist_id;
    let site = cmd.site.unwrap_or_default();

    let cookie = find_cookie(site).await;
    let proxy = select_proxy(cmd.no_proxy).await;

    let search = if cmd.watching {
        super::get_from_watching_list(client_id).await?
    } else {
        super::get_from_input(cmd.entries).await?
    };

    let (vec_anime, referrer) = Scraper::new(proxy, cookie)
        .run(search.into_iter(), site)
        .await?;

    let referrer = referrer.unwrap_or_default();
    let (cmd, cmd_referrer) = match which("mpv") {
        Ok(c) => (c, format!("--referrer={referrer}")),
        _ => (
            which("vlc").unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\VideoLAN\VLC\vlc")),
            format!("--http-referrer={referrer}"),
        ),
    };

    let mut episodes = vec![];
    let mut ids = HashMap::new();
    for anime in &vec_anime {
        ids.insert(&anime.url, anime.id);
        episodes.extend(tui::episodes_choice(anime)?);
    }

    let mut child = Command::new(cmd)
        .arg(&cmd_referrer)
        .arg("-v")
        .args(&episodes)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        let stdout = BufReader::new(child.stdout.as_mut().unwrap());
        let stderr = BufReader::new(child.stderr.as_mut().unwrap());
        let stdout_lines = LinesStream::new(stdout.lines());
        let stderr_lines = LinesStream::new(stderr.lines());

        let mut merged = tokio_stream::StreamExt::merge(stdout_lines, stderr_lines);

        let mut progress = Progress::new(Anilist::new(client_id)?);
        while let Some(Ok(line)) = merged.next().await {
            if line.contains("Opening done") {
                let url = line.split_whitespace().last().unwrap();
                let num = parse_number(url);
                let origin = parse_url(url, num);

                progress.anime_id(ids.get(&origin).copied().flatten());
                progress.episode(num.map(|n| n.value));
            } else if line.contains('%') && !line.contains("(Paused)") {
                progress.percentage(parse_percentage(&line));
            }

            progress.update().await;
        }
    }

    let _ = child.wait().await?;

    Ok(())
}
