use super::*;

use colored::Colorize;
use std::io::prelude::*;

pub struct Choice {
    link: String,
    name: String,
}

impl Choice {
    pub fn from(link: String, name: String) -> Self {
        Self { link, name }
    }
}

pub fn get_choice(choices: Vec<Choice>) -> Result<Vec<String>> {
    Ok(match choices.len() {
        0 => bail!("No match found"),
        1 => vec![choices[0].link.to_string()],
        _ => {
            println!(
                "{}",
                format!("{} results found\n", choices.len())
                    .bright_cyan()
                    .bold()
            );
            for i in 0..choices.len() {
                println!(
                    "[{}] {}",
                    format!("{}", i + 1).bright_purple(),
                    format!("{}", choices[i].name).bright_green()
                );
            }

            print!(
                "\n{}{}\n{}",
                format!("==> ").bright_red().bold(),
                format!("What to watch (eg: 1 2 3 or 1-3) [default=All]").bold(),
                format!("==> ").bright_red().bold()
            );
            std::io::stdout().flush()?;

            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;

            let re = Regex::new(r"[^\d]").unwrap();
            let mut multi = re
                .replace_all(&line, " ")
                .split_ascii_whitespace()
                .into_iter()
                .map(|v| v.parse().unwrap_or(1) as usize)
                .filter(|i| i.gt(&0) && i.le(&choices.len()))
                .collect::<Vec<_>>();

            if line.contains('-') {
                let re = Regex::new(r"(?:\d+\-\d*)").unwrap();
                re.captures_iter(&line)
                    .map(|c| c[0].to_string())
                    .for_each(|s| {
                        let range = s
                            .split('-')
                            .into_iter()
                            .map(|v| v.parse().unwrap_or(choices.len()) as usize)
                            .filter(|i| i.gt(&0) && i.le(&choices.len()))
                            .collect::<Vec<_>>();
                        let start = range.first().unwrap();
                        let end = range.last().unwrap();

                        multi.extend((start + 1..end + 1).collect::<Vec<_>>())
                    });
            }

            multi.sort();
            multi.dedup();
            let res = multi
                .iter()
                .map(|i| choices[i - 1].link.to_string())
                .collect::<Vec<_>>();

            match res.len() {
                0 => choices.into_iter().map(|c| c.link).collect::<Vec<_>>(),
                _ => res,
            }
        }
    })
}

pub fn get_token(url: &str) -> Result<String> {
    print!(
        "{}\n\n{}\n{}\n\n{}{}\n{}",
        format!("Anilist Oauth").bright_cyan().bold(),
        format!("Autenticate to: ").bright_green(),
        format!("{}", url).bright_purple().bold(),
        format!("==> ").bright_red().bold(),
        format!("Paste token here: ").bold(),
        format!("==> ").bright_red().bold()
    );
    std::io::stdout().flush()?;

    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_string())
}

pub fn format_err(s: anyhow::Error) -> colored::ColoredString {
    format!("[ERR] {}", s).red()
}
