use clap::Parser;

use std::path::PathBuf;

use super::Site;
use crate::parser;
use crate::parser::InfoNum;
use crate::range::Range;
use crate::scraper::select_proxy;
use crate::tui;

use anyhow::{ensure, Result};
use futures::stream::StreamExt;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use reqwest::Client;
use tokio::{fs, io::AsyncWriteExt};
use tokio_stream as stream;

/// Donwload anime
#[derive(Parser, Debug, Default)]
#[command(arg_required_else_help(true))]
pub struct Args {
    /// Source urls or scraper's queries
    #[arg(required_unless_present("watching"))]
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

    let bars = tui::Bars::new();
    let mut pool = vec![];

    for mut anime in vec_anime.into_iter() {
        anime.range(cmd.range);

        if cmd.interactive {
            tui::episodes_choice(&mut anime)?;
        } else {
            anime.expand();
        }

        let mut parent = cmd.dir.clone();
        if cmd.auto_dir {
            let name = parser::parse_name(&anime.info.url)?;
            let dir = to_snake_case!(name);

            parent.push(dir);
        }

        for (i, url) in anime.episodes.into_iter().enumerate() {
            let pb = bars.add_bar();
            let mut path = parent.clone();
            let info = anime.info.clone();

            let future = async move {
                let client = Client::new();
                let filename = parser::parse_filename(&url)?;
                let source_size = client
                    .head(&url)
                    .header(REFERER, referrer.unwrap_or_default())
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
                        .append(!cmd.force)
                        .truncate(cmd.force)
                        .write(cmd.force)
                        .create(true)
                        .open(path)
                        .await?
                };

                let file_size = dest.metadata().await?.len();
                ensure!(file_size < source_size, filename + " already exists");

                let msg = match (info.num, info.episodes) {
                    (Some(InfoNum { value, alignment }), Some(Range { start, .. })) => {
                        "Ep. ".to_string()
                            + &zfill!(value + start + i as u32, alignment)
                            + " "
                            + &info.name
                    }
                    _ => info.name.clone(),
                };

                pb.set_position(file_size);
                pb.set_length(source_size);
                pb.set_message(msg);

                let mut source = client
                    .get(url)
                    .header(RANGE, format!("bytes={}-", file_size))
                    .header(REFERER, referrer.unwrap_or_default())
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
        .buffer_unordered(cmd.dim_buff.max(1))
        .collect::<Vec<_>>()
        .await;

    Ok(())
}
