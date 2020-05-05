mod anime;
mod cli;
mod tasks;
mod utils;

use crate::anime::Anime;
use crate::cli::Cli;
use crate::tasks::Tasks;
use crate::utils::extract_name;

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
        let default_path = args.dir.last().unwrap().to_owned();

        let path = if args.auto_dir {
            let mut path = default_path;
            let name = match extract_name(&url) {
                Ok(n) => Some(n),
                Err(e) => {
                    eprintln!("{}", format!("[ERROR] {}", e).red());
                    None
                }
            };

            path.push(name.unwrap());
            path.to_owned()
        } else {
            match i >= args.dir.len() {
                true => default_path,
                _ => args.dir[i].to_owned(),
            }
        };

        let opts = (args.start, args.end, args.auto_episode);
        match Anime::new(url, path, opts) {
            Ok(a) => all_anime.push(a),
            Err(e) => eprintln!("{}", format!("[ERROR] {}", e).red()),
        }
    }

    let mut tasks = Tasks::new();
    for anime in &all_anime {
        let urls = match anime.url_episodes() {
            Ok(u) => Some(u),
            Err(e) => {
                eprintln!("{}", format!("[ERROR] {}", e).red());
                None
            }
        };

        for url in urls.unwrap() {
            let pb = ProgressBar::new(0);
            pb.set_style(sty.clone());

            let opts = (anime.path(), args.force, m.add(pb));

            tasks.add(thread::spawn(move || {
                thread::park();
                Anime::download(&url, &opts)
            }));
        }
    }

    thread::spawn(move || m.join().unwrap());

    tasks.unpark_and_join(args.max_threads)
}
