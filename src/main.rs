#[macro_use]
mod utils;

mod anime;
mod cli;

use crate::anime::{Anime, Scraper, Site};
use crate::cli::Cli;
use crate::utils::*;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use threadpool::ThreadPool;

use std::thread;

fn main() {
    let args = Cli::new();
    let mut anime_urls = args.urls;

    let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template(
            "{spinner:.green} [{elapsed}] [{bar:35.cyan/blue}] {bytes}/{total_bytes} ({eta}) {wide_msg}",
        )
        .progress_chars("#>-");

    // for flickering bar bug (https://github.com/mitsuhiko/indicatif/issues/143)
    m.set_move_cursor(cfg!(windows));

    // Download only from first given url
    if args.single {
        let pb = ProgressBar::new(0);
        pb.set_style(sty.clone());

        let opts = (args.dir.last().unwrap().to_owned(), args.force, pb);
        return unwrap_err!(Anime::download(&anime_urls[0], &opts));
    }

    // Scrape from animeworld.tv and find correct urls
    if args.aw {
        let query = anime_urls.join("+");
        let url = unwrap_err!(Scraper::new(Site::AnimeWord, query).run());

        anime_urls = vec![url]
    }

    let pool = ThreadPool::new(args.max_threads);
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

        let opts = (args.start, args.end, args.auto_episode);
        let anime = unwrap_err!(Anime::new(url, path, opts));

        let urls = unwrap_err!(anime.url_episodes());
        for url in urls {
            let pb = ProgressBar::new(0);
            pb.set_style(sty.clone());

            let opts = (anime.path(), args.force, m.add(pb));
            pool.execute(move || unwrap_err!(Anime::download(&url, &opts)));
        }
    }

    let bars = thread::spawn(move || m.join().unwrap());

    pool.join();
    bars.join().unwrap();
}
