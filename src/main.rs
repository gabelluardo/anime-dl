mod anime;
mod cli;
mod tasks;
mod utils;

use crate::anime::Anime;
use crate::cli::Cli;
use crate::tasks::Tasks;

use colored::Colorize;
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

        let path = match i >= args.dir.len() {
            true => args.dir.last().unwrap().to_owned(),
            _ => args.dir[i].to_owned(),
        };

        match Anime::new(url, args.start, args.end, path) {
            Ok(a) => all_anime.push(a),
            Err(e) => eprintln!("{}", format!("[ERROR] {}", e).red()),
        }
    }

    let mut tasks = Tasks::new();
    for anime in &all_anime {
        let urls = match anime.url_episodes(args.auto) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("{}", format!("[ERROR] {}", e).red());
                vec![]
            }
        };

        for url in urls {
            let path = anime.path();
            let force = args.force.to_owned();
            let pb = m.add(ProgressBar::new(0));
            pb.set_style(sty.clone());

            tasks.add(thread::spawn(move || {
                thread::park();
                Anime::download(&url, &path, &force, &pb)
            }));
        }
    }

    thread::spawn(move || m.join().unwrap());

    tasks.join(args.max_threads)
}
