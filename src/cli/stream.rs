use clap::Parser;

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Result;
use tokio::process::Command;
use which::which;

use super::Site;
use crate::scraper::select_proxy;
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
    let site = cmd.site.unwrap_or_default();
    let proxy = select_proxy(cmd.no_proxy).await;

    let (vec_anime, referrer) = if cmd.watching {
        super::get_from_watching_list(cmd.anilist_id, proxy, site).await?
    } else {
        super::get_from_input(cmd.entries, proxy, site).await?
    };

    let referrer = referrer.unwrap_or_default();
    let (cmd, cmd_referrer) = match which("mpv") {
        Ok(c) => (c, format!("--referrer={referrer}")),
        _ => (
            which("vlc").unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\VideoLAN\VLC\vlc")),
            format!("--http-referrer={referrer}"),
        ),
    };

    let mut episodes = vec![];
    for mut anime in vec_anime.into_iter() {
        tui::episodes_choice(&mut anime)?;
        episodes.extend(anime.episodes);
    }

    Command::new(cmd)
        .arg(&cmd_referrer)
        .args(episodes)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}
