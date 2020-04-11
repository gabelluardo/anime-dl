mod anime;
mod cli;
mod utils;

use crate::anime::{Anime, Error};
use cli::Cli;

use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use std::thread::{spawn, JoinHandle};

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
        let path = args.dir[i].clone();

        match Anime::new(url, args.start, args.end, path) {
            Ok(a) => all_anime.push(a),
            Err(e) => println!("{}", format!("[ERROR] {}", e).red()),
        }
    }

    let mut tasks: Vec<JoinHandle<Error<String>>> = vec![];
    for anime in &all_anime {
        let mut urls = vec![];

        match anime.url_episodes(args.auto) {
            Ok(u) => urls = u,
            Err(e) => println!("{}", format!("[ERROR] {}", e).red()),
        }

        for url in urls {
            let path = anime.path();
            let force = args.force.clone();
            let pb = m.add(ProgressBar::new(0));
            pb.set_style(sty.clone());

            tasks.push(spawn(move || {
                std::thread::park();
                Anime::download(&url, &path, &force, &pb)
            }));
        }
    }

    let progress = spawn(move || m.join().unwrap());

    let mut active: Vec<JoinHandle<Error<String>>> = vec![];
    for _ in 0..tasks.len() {
        let t = tasks.remove(0);

        t.thread().unpark();
        active.push(t);

        if active.len() >= args.max_threads {
            print_result(active.remove(0));
        }
    }

    progress.join().unwrap();
}

fn print_result(t: JoinHandle<Error<String>>) {
    match t.join().unwrap() {
        Ok(_) => (),
        Err(e) => println!("{}", format!("[ERROR] {}", e).red()),
    }
}
