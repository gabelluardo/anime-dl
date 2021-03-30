use std::io::Write;

use bunt::{print, println};
use tokio::io::{self, AsyncBufReadExt, BufReader};

use super::*;

pub struct Choice {
    link: String,
    name: String,
}

impl Choice {
    pub fn new(link: String, name: String) -> Self {
        Self { link, name }
    }
}

pub async fn get_choice(choices: Vec<Choice>) -> Result<Vec<String>> {
    match choices.len() {
        0 => bail!("No match found"),
        1 => Ok(vec![choices[0].link.to_string()]),
        _ => {
            println!("{$cyan+bold}{} results found{/$}\n", choices.len());
            for (i, c) in choices.iter().enumerate() {
                println!("[{[magenta]}] {[green]}", i + 1, c.name);
            }

            print!(
                "\n\
                {$red}==> {/$}\
                {$bold}What to watch (eg: 1 2 3 or 1-3) [default=All]{/$}\n\
                {$red}==> {/$}",
            );
            // TODO: Fix console bug on async flush
            std::io::stdout().flush()?;

            let mut reader = BufReader::new(io::stdin());
            let mut line = String::new();
            reader.read_line(&mut line).await?;

            let re = Regex::new(r"[^\d]").unwrap();
            let mut multi = re
                .replace_all(&line, " ")
                .split_ascii_whitespace()
                .into_iter()
                .filter_map(|v| v.parse().ok())
                .collect::<Vec<_>>();

            if line.contains(&[',', '-', '.'][..]) {
                let re = Regex::new(r"(?:\d+[,\-.]+\d*)").unwrap();
                re.captures_iter(&line)
                    .map(|c| c[0].to_string())
                    .for_each(|s| {
                        multi.extend(
                            Range::<usize>::parse_and_fill(&s, choices.len())
                                .unwrap()
                                .expand(),
                        )
                    })
            }

            multi.sort_unstable();
            multi.dedup();

            let urls = match multi.len() {
                0 => choices.into_iter().map(|c| c.link).collect::<Vec<_>>(),
                _ => multi
                    .into_iter()
                    .filter_map(|i| choices.get(i - 1))
                    .map(|c| c.link.to_string())
                    .collect::<Vec<_>>(),
            };

            if urls.is_empty() {
                bail!("No episode found")
            }

            Ok(urls)
        }
    }
}

#[cfg(feature = "anilist")]
pub async fn get_token(url: &str) -> Result<String> {
    print!(
        "{$cyan+bold}Anilist Oauth{/$}\n\n\
        {$green}Authenticate to: {/$}\n\
        {[magenta+bold]}\n\n\
        {$red}==> {/$}\
        {$bold}Paste token here: {/$}\n\
        {$red}==> {/$}",
        url
    );
    std::io::stdout().flush()?;

    let mut reader = BufReader::new(io::stdin());
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    Ok(line.trim().to_string())
}
