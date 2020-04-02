mod anime;
mod cli;
mod utils;

use crate::anime::{Anime, Error};
use cli::Cli;

use exitfailure::ExitFailure;

fn main() -> Result<(), ExitFailure> {
    let args = Cli::new();
    // println!("{:?}", args);

    // let urls = vec![
    //     String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_12_SUB_ITA.mp4")
    //     ];

    let mut all_anime: Vec<Anime> = vec![];
    for i in 0..args.urls.len() {
        let url = &args.urls[i];
        let path = args.dir[i].clone();

        all_anime.push(Anime::new(url, args.start, path)?);
    }

    let mut tasks: Vec<std::thread::JoinHandle<Error<()>>> = vec![];
    for anime in &all_anime {
        for url in anime.url_episodes()? {
            let path = anime.path()?;

            tasks.push(std::thread::spawn(move || Anime::download(&url, &path)));
        }
    }

    for t in tasks {
        t.join().unwrap()?;
    }

    Ok(())
}
