mod anime;
mod cli;

use anime::Anime;
use anime::Error;
use cli::Cli;

use exitfailure::ExitFailure;
use futures::future::join_all;

fn main() -> Result<(), ExitFailure> {
    let args = Cli::new();
    println!("{:?}", args);

    let urls = vec![
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_01_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_02_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_03_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_04_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_05_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_06_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_07_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_08_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_09_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_10_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_11_SUB_ITA.mp4"),
        String::from("http://eurybia.feralhosting.com/animeworlds3/DDL/ANIME/IshuzokuReviewers/IshuzokuReviewers_Ep_12_SUB_ITA.mp4")
        ];

    let anime = Anime::new(urls)?;

    if !args.asyn {
        let mut t_tasks: Vec<std::thread::JoinHandle<Error<()>>> = vec![];

        for url in anime.url_episodes()? {
            t_tasks.push(std::thread::spawn(move || Anime::download(&url)));
        }

        println!("{:#?}", anime);

        for t in t_tasks {
            let _ = t.join();
        }
    } else {
        let mut rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let mut _tasks: Vec<tokio::task::JoinHandle<Error<()>>> = vec![];

            for url in anime.url_episodes() {
                println!("{:#?}", url);

                // tasks.push(tokio::task::spawn(Anime::async_download(&url)));
            }
            join_all(_tasks).await;
        })
    }

    Ok(())
}

// fn find_path(path: &PathBuf) -> Error<String> {
//     let content = std::fs::read_to_string(path)
//         .with_context(|_| format!("Could not read file `{}`", path.display()))?;

//     Ok(content)
// }

// fn find_matches(content: &str, pattern: &str, mut writer: impl std::io::Write) -> Error<()> {
//     for line in content.lines() {
//         let curr_line = line;
//         if curr_line.contains(pattern) {
//             writeln!(writer, "{}", curr_line).with_context(|_| format!("Could print on writer"))?;
//         }
//     }

//     Ok(())
// }
