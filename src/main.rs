mod anime;
mod cli;
mod utils;

use crate::anime::{Anime, Error};

use colored::Colorize;
use std::thread::{spawn, JoinHandle};

use cli::Cli;

fn main() {
    let args = Cli::new();
    // println!("{:?}", args);

    let mut all_anime: Vec<Anime> = vec![];
    for i in 0..args.urls.len() {
        let url = &args.urls[i];
        let path = args.dir[i].clone();

        match Anime::new(url, args.start, path) {
            Ok(a) => all_anime.push(a),
            Err(e) => println!("{}", format!("{}", e).red()),
        }
    }

    let mut tasks: Vec<JoinHandle<Error<String>>> = vec![];
    for anime in &all_anime {
        for url in anime.url_episodes() {
            let path = anime.path();

            tasks.push(spawn(move || Anime::download(&url, &path)));
        }
    }

    for t in tasks {
        match t.join().unwrap() {
            Ok(s) => println!("{}", format!("Completed `{}`", s).green()),
            Err(e) => println!("{}", format!("{}", e).red()),
        }
    }
}
