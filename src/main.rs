#[macro_use]
mod utils;

mod anime;
mod cli;

use crate::anime::{Anime, Scraper};
use crate::cli::Cli;
use crate::utils::*;

use futures::future::join_all;

#[tokio::main]
async fn main() {
    let args = Cli::new();
    let (multi_bars, style) = instance_multi_bars();

    let (start, end) = match args.range {
        Some(range) => range.extract(),
        _ => (1, 0),
    };

    // Scrape form archive and find correct url
    let anime_urls = match args.search {
        Some(site) => {
            let query = args.urls.join("+");
            vec![unwrap_err!(Scraper::new(site, query).run().await)]
        }
        _ => args.urls,
    };

    // Download only from first given url
    if args.single {
        let pb = instance_bar(&style);
        let opts = (args.dir.last().unwrap().to_owned(), args.force, pb);

        return unwrap_err!(Anime::download(anime_urls[0].clone(), opts).await);
    }

    // TODO: Limit max parallel tasks with `args.max_thread`
    let mut pool: Vec<tokio::task::JoinHandle<()>> = vec![];
    for i in 0..anime_urls.len() {
        let url = &anime_urls[i];
        let default_path = args.dir.last().unwrap().to_owned();

        let path = if args.auto_dir {
            let mut path = default_path;
            let info = unwrap_err!(extract_info(&url));

            path.push(info.name);
            path.to_owned()
        } else {
            match i >= args.dir.len() {
                true => default_path,
                _ => args.dir[i].to_owned(),
            }
        };

        let opts = (start, end, args.auto_episode);
        let anime = unwrap_err!(Anime::new(url, path, opts));

        let urls = unwrap_err!(anime.url_episodes().await);
        for url in urls {
            let pb = instance_bar(&style);
            let opts = (anime.path(), args.force, multi_bars.add(pb));

            pool.push(tokio::spawn(async move {
                unwrap_err!(Anime::download(url, opts).await)
            }));
        }
    }

    let bars = tokio::task::spawn_blocking(move || multi_bars.join().unwrap());

    join_all(pool).await;
    bars.await.unwrap();
}
