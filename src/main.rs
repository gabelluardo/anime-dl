mod anime;
mod cli;
mod utils;

use crate::anime::{Anime, Error};

use colored::Colorize;
use std::thread::{spawn, JoinHandle};

use cli::Cli;

fn main() {
    let args = Cli::new();
    // println!("{:#?}", args);

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

            tasks.push(spawn(move || Anime::download(&url, &path)));

            if tasks.len() >= args.max_threads {
                print_result(tasks.remove(0));
            }
        }
    }

    for t in tasks {
        print_result(t);
    }
}

fn print_result(t: JoinHandle<Error<String>>) {
    match t.join().unwrap() {
        Ok(s) => println!("{}", format!("[INFO] Completed {}", s).green()),
        Err(e) => println!("{}", format!("[ERROR] {}", e).red()),
    }
}
