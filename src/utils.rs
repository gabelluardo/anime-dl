use crate::Error;

use failure::bail;
use regex::Regex;

pub const REGEX_VALUE: &str = "_{}_";
pub const CHUNK_SIZE: usize = 1024 * 1024; // 1024^2 = 1MB

pub fn extract(url: &str) -> Error<(String, u32)> {
    let re = Regex::new(r"_\d+_")?;
    let cap = match re.captures(url) {
        Some(c) => c,
        None => bail!("Unable parse `{}`", url),
    };

    let url = re.replace_all(url, REGEX_VALUE).to_string();
    let last: u32 = cap
        .get(cap.len() - 1)
        .map(|c| c.as_str())
        .unwrap()
        .replace("_", "")
        .parse()?;

    Ok((url, last))
}

pub fn fix_num_episode(num: u32) -> String {
    format!("_{:02}_", num)
}

// fn find_matches(content: &str, pattern: &str, mut writer: impl std::io::Write) -> Error<()> {
//     for line in content.lines() {
//         let curr_line = line;
//         if curr_line.contains(pattern) {
//             writeln!(writer, "{}", curr_line).with_context(|_| format!("Could print on writer"))?;
//         }
//     }

//     Ok(())
// }
//
// ==========================================================================================

// download in async mode [WIP]
// #[structopt(short = "a", long = "async")]
// pub asyn: bool,

// ==========================================================================================
// use futures::future::join_all;
//
// let mut rt = tokio::runtime::Runtime::new()?;
// rt.block_on(async {
//     let mut _tasks: Vec<tokio::task::JoinHandle<Error<()>>> = vec![];
//     for anime in &all_anime {
//         for url in anime.url_episodes().unwrap() {
//             let _path = anime.path().unwrap();
//             let url = url.clone();
//             _tasks.push(tokio::task::spawn(move |_| Anime::_async_download(&url)));
//         }
//     }
//     join_all(_tasks).await;
// })
//
// ===========================================================================================
// use futures::stream::TryStreamExt;
// use tokio_util::compat::FuturesAsyncReadCompatExt;
//
// pub async fn _async_download(url: &str) -> Error<()> {
//     let r_url = Url::parse(url)?;
//     let filename = r_url
//         .path_segments()
//         .and_then(|segments| segments.last())
//         .unwrap_or("tmp.bin");

//     let client = reqwest::Client::new();
//     let response = client.head(url).send().await?;

//     let total_size: u64 = response
//         .headers()
//         .get(CONTENT_LENGTH)
//         .and_then(|ct_len| ct_len.to_str().ok())
//         .and_then(|ct_len| ct_len.parse().ok())
//         .unwrap_or(0);

//     let mut outfile = tokio::fs::File::create(format!("prova/{}", filename)).await?;

//     println!(
//         "---\nDownloading {}\nsize = {:?}MB -- {:?}B",
//         filename,
//         total_size / 1024u64.pow(2),
//         total_size,
//     );

//     for range in PartialRangeIter::new(0, total_size - 1, CHUNK_SIZE)? {
//         let response = client
//             .get(url)
//             .header(RANGE, range)
//             .send()
//             .await?
//             .error_for_status()?;

//         let response = response.bytes_stream();
//         let response = response
//             .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
//             .into_async_read();

//         let mut response = response.compat();

//         // println!("range {:?}MB -- {:?}B", range / 1024u64.pow(2), range);
//         tokio::io::copy(&mut response, &mut outfile).await?;
//     }

//     Ok(())
// }
