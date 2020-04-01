mod cli;
use cli::Cli;
use exitfailure::ExitFailure;
use futures::future::join_all;
use reqwest;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::Url;
use std::thread;
use tokio::task;
use tokio::task::JoinHandle;

mod my_async;
mod range;
use range::Error;

#[tokio::main]
async fn main() -> Result<(), ExitFailure> {
    // fn main() -> Error<()> {
    let _args = Cli::new();
    // println!("{:?}", args);

    let urls = vec![
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_01_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_02_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_03_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_04_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_05_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_06_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_07_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_08_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_09_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_10_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_11_SUB_ITA.mp4",
        "http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_12_SUB_ITA.mp4"
        ];

    let mut tasks: Vec<JoinHandle<Error<()>>> = vec![];
    // let mut t_tasks: Vec<thread::JoinHandle<Error<()>>> = vec![];
    for url in urls {
        tasks.push(task::spawn(my_async::as_download_episode(url)));
        // t_tasks.push(thread::spawn(move || download_episode(url)));
    }

    join_all(tasks).await;
    // for t in t_tasks {
    //     let _ = t.join();
    // }
    Ok(())
}

fn download_episode(url: &str) -> Error<()> {
    // 1024^2 = 1MB
    const CHUNK_SIZE: usize = 1024 * 1024;

    let r_url = Url::parse(url)?;
    let filename = r_url
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("tmp.bin");

    let client = reqwest::blocking::Client::new();
    let response = client.head(url).send()?;

    let total_size: u64 = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok())
        .unwrap_or(0);

    let mut outfile = std::fs::File::create(format!("prova/{}", filename))?;

    println!(
        "---\nDownloading {}\nsize = {:?}MB -- {:?}B",
        filename,
        total_size / 1024u64.pow(2),
        total_size,
    );

    for range in range::PartialRangeIter::new(0, total_size - 1, CHUNK_SIZE)? {
        let mut response = client
            .get(url)
            .header(RANGE, range)
            .send()?
            .error_for_status()?;

        std::io::copy(&mut response, &mut outfile)?;
    }

    Ok(())
}
