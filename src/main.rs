#[macro_use]
mod macros;

mod anime;
mod cli;
mod tasks;
mod utils;

use crate::anime::Anime;
use crate::cli::Cli;
use crate::tasks::TaskPool;
use crate::utils::extract_name;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use std::thread;

fn main() {
    let args = Cli::new();

    let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template(
            "{spinner:.green} [{elapsed}] [{bar:35.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}",
        )
        .progress_chars("#>-");

    let mut all_anime: Vec<Anime> = vec![];
    for i in 0..args.urls.len() {
        let url = &args.urls[i];
        let default_path = args.dir.last().unwrap().to_owned();

        let path = if args.auto_dir {
            let mut path = default_path;
            let name = unwrap_err!(extract_name(&url));

            path.push(name);
            path.to_owned()
        } else {
            match i >= args.dir.len() {
                true => default_path,
                _ => args.dir[i].to_owned(),
            }
        };

        let opts = (args.start, args.end, args.auto_episode);
        let new_anime = unwrap_err!(Anime::new(url, path, opts));

        all_anime.push(new_anime);
    }

    let mut tasks = TaskPool::new(args.max_threads);
    for anime in &all_anime {
        let urls = unwrap_err!(anime.url_episodes());

        for url in urls {
            let pb = ProgressBar::new(0);
            pb.set_style(sty.clone());

            let opts = (anime.path(), args.force, m.add(pb));

            tasks.add(thread::spawn(move || {
                thread::park();
                Anime::download(&url, &opts)
            }));
        }
    }

    let bars = thread::spawn(move || m.join().unwrap());

    tasks.unpark_and_join();
    bars.join().unwrap();
}
