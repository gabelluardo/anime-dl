#[macro_use]
mod macros;

mod anime;
mod cli;
mod scraper;
mod utils;

use crate::anime::Anime;
use crate::cli::Cli;
use crate::scraper::Scraper;
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

    // Scrape from archive and find correct url
    let anime_urls = match args.search {
        Some(site) => {
            let query = args.urls.join("+");
            print_err!(Scraper::new(site, query).run().await)
        }
        _ => args.urls,
    };

    // Download only from first given url
    if args.single {
        let pb = instance_bar(&style);
        let opts = (args.dir.last().unwrap().to_owned(), args.force, pb);

        return print_err!(Anime::download(anime_urls[0].clone(), opts).await);
    }

    // TODO: Limit max parallel tasks with `args.max_thread`
    let mut pool: Vec<tokio::task::JoinHandle<()>> = vec![];
    for url in &anime_urls {
        let mut dir = args.dir.last().unwrap().to_owned();

        let path = if args.auto_dir {
            let subfolder = print_err!(extract_info(&url));

            dir.push(subfolder.name);
            dir
        } else {
            let pos = anime_urls
                .iter()
                .map(|u| u.as_str())
                .position(|u| u == url)
                .unwrap();

            match args.dir.get(pos) {
                Some(path) => path.to_owned(),
                _ => dir,
            }
        };

        let opts = (start, end, args.auto_episode);
        let anime = print_err!(Anime::new(url, path, opts));

        let urls = print_err!(anime.url_episodes().await);
        for url in urls {
            let pb = instance_bar(&style);
            let opts = (anime.path(), args.force, multi_bars.add(pb));

            pool.push(tokio::spawn(async move {
                print_err!(Anime::download(url, opts).await)
            }));
        }
    }

    let bars = tokio::task::spawn_blocking(move || multi_bars.join().unwrap());

    join_all(pool).await;
    bars.await.unwrap();
}
