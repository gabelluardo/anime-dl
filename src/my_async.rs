// mod cli;
// use cli::Cli;
use super::range;
use super::range::Error;

use futures::stream::TryStreamExt;
use reqwest;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::Url;
use tokio_util::compat::FuturesAsyncReadCompatExt;

pub async fn as_download_episode(url: &str) -> Error<()> {
    const CHUNK_SIZE: usize = 1024 * 1024;

    let r_url = Url::parse(url)?;
    let filename = r_url
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("tmp.bin");

    let client = reqwest::Client::new();
    let response = client.head(url).send().await?;

    let total_size: u64 = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok())
        .unwrap_or(0);

    let mut outfile = tokio::fs::File::create(format!("prova/{}", filename)).await?;

    println!(
        "---\nDownloading {}\nsize = {:?}MB -- {:?}B",
        filename,
        total_size / 1024u64.pow(2),
        total_size,
    );

    for range in range::PartialRangeIter::new(0, total_size - 1, CHUNK_SIZE)? {
        let response = client
            .get(url)
            .header(RANGE, range)
            .send()
            .await?
            .error_for_status()?;

        let response = response.bytes_stream();
        let response = response
            .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
            .into_async_read();

        let mut response = response.compat();

        // println!("range {:?}MB -- {:?}B", range / 1024u64.pow(2), range);
        tokio::io::copy(&mut response, &mut outfile).await?;
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
