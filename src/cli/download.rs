use clap::Parser;

use std::path::PathBuf;
use std::sync::Arc;

use super::Site;
use crate::parser::InfoNum;
use crate::parser::{self, parse_number};
use crate::range::Range;
use crate::scraper::{Scraper, find_cookie, select_proxy};
use crate::tui::Tui;

use anyhow::{Result, ensure};
use futures::stream::StreamExt;
use reqwest::Client;
use reqwest::header::{CONTENT_LENGTH, RANGE, REFERER};
use tokio::{fs, io::AsyncWriteExt};
use tokio_stream as stream;

/// Donwload anime
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

pub async fn execute(args: Args) -> Result<()> {
    let client_id = args.anilist_id;
    let site = args.site.unwrap_or_default();

    let cookie = find_cookie(site).await;
    let proxy = select_proxy(args.no_proxy).await;

    let search = if args.watching || args.entries.is_empty() {
        super::get_from_watching_list(client_id).await?
    } else {
        super::get_from_input(args.entries).await?
    };

    let (vec_anime, referrer) = Scraper::new(proxy, cookie)
        .run(search.into_iter(), site)
        .await?;

    let ui = Tui::new();
    let client = Arc::new(Client::new());
    let mut pool = vec![];
    for anime in &vec_anime {
        let episodes = match args.range {
            Some(range) if !args.interactive => anime.select_from_range(range),
            _ => Tui::select_episodes(anime)?,
        };

        let mut parent = args.dir.clone();
        if args.auto_dir {
            let name = parser::parse_name(&anime.url)?;
            let dir = to_snake_case!(name);

            parent.push(dir);
        }

        for url in episodes {
            let pb = ui.add_bar();
            let mut path = parent.clone();
            let client = client.clone();

            let future = async move {
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
                        .append(!args.force)
                        .truncate(args.force)
                        .write(args.force)
                        .create(true)
                        .open(path)
                        .await?
                };

                let file_size = dest.metadata().await?.len();
                ensure!(file_size < source_size, filename + " already exists");

                let msg = match parse_number(&url) {
                    Some(InfoNum { value, alignment }) => gen_msg!(value, alignment, anime.name),
                    _ => anime.name.clone(),
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
        .buffer_unordered(args.dim_buff.max(1))
        .collect::<Vec<_>>()
        .await;

    Ok(())
}
