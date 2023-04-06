use crate::errors::{Quit, RemoteError, UserError};
use crate::range::Range;

use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use rustyline::{config::Configurer, error::ReadlineError, ColorMode, DefaultEditor};

#[derive(Clone)]
pub struct Choice {
    link: String,
    name: String,
}

impl Choice {
    pub fn new(link: String, name: String) -> Self {
        Self { link, name }
    }
}

fn parse_input(line: String, choices: &[Choice]) -> Vec<String> {
    let line = line
        .replace(&[',', '.'][..], " ")
        .chars()
        .filter(|c| c.is_ascii_digit() || c.is_ascii_whitespace() || *c == '-')
        .collect::<String>();
    let sel = line
        .split_ascii_whitespace()
        .map(|s| s.trim())
        .collect::<Vec<_>>();
    let mut selected = vec![];
    for s in sel {
        if let Ok(num) = s.parse::<usize>() {
            selected.push(num);
        } else if let Ok(range) = Range::<usize>::parse_and_fill(s, choices.len()) {
            selected.extend(range.expand())
        }
    }
    selected.sort_unstable();
    selected.dedup();
    match selected.len() {
        0 => choices
            .iter()
            .map(|c| c.link.to_string())
            .collect::<Vec<_>>(),
        _ => selected
            .iter()
            .filter_map(|i| choices.get(i - 1))
            .map(|c| c.link.to_string())
            .collect::<Vec<_>>(),
    }
}

pub fn get_choice(choices: &[Choice], query: Option<String>) -> Result<Vec<String>> {
    match choices.len() {
        0 => bail!(UserError::Choices),
        1 => Ok(vec![choices[0].link.to_string()]),
        _ => {
            let len = choices.len();
            let name = query.map(|n| format!(" for `{n}`")).unwrap_or_default();
            let results = format!("{len} results found{name}");
            println!("{}\n", results.cyan().bold());
            for (i, c) in choices.iter().enumerate() {
                println!("[{}] {}", (i + 1).magenta(), c.name.green());
            }
            println!();
            println!(
                "{} {}",
                "::".red(),
                "Make your selection (eg: 1 2 3 or 1-3) [default=All, <q> for exit]".bold()
            );
            let mut rl = DefaultEditor::new().context(UserError::InvalidInput)?;
            rl.set_color_mode(ColorMode::Enabled);
            let prompt = "~❯ ".red().to_string();
            let urls = match rl.readline(&prompt) {
                Ok(line) => {
                    if line.contains('q') {
                        bail!(Quit)
                    }
                    parse_input(line, choices)
                }
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                    bail!(Quit)
                }
                Err(_) => {
                    bail!(UserError::InvalidInput);
                }
            };
            println!();
            if urls.is_empty() {
                bail!(RemoteError::EpisodeNotFound);
            }
            Ok(urls)
        }
    }
}

#[cfg(feature = "anilist")]
pub fn get_token(url: &str) -> Result<String> {
    let action = "Authenticate to:".green();
    let input = format!("{} {}", "::".red(), "Paste token here:".bold());
    println!(
        "{}\n\n\
        {action} {}\n\n\
        {input}",
        "Anilist Oauth".cyan().bold(),
        url.magenta().bold()
    );
    let mut rl = DefaultEditor::new().context(UserError::InvalidInput)?;
    let prompt = "~❯ ".red().to_string();
    let line = rl
        .readline(&prompt)
        .map(|s| s.trim().to_string())
        .context(UserError::InvalidInput)?;
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_input() {
        let choices = vec![
            Choice {
                link: "link1".to_string(),
                name: "choice1".to_string(),
            },
            Choice {
                link: "link2".to_string(),
                name: "choice2".to_string(),
            },
            Choice {
                link: "link3".to_string(),
                name: "choice3".to_string(),
            },
            Choice {
                link: "link4".to_string(),
                name: "choice4".to_string(),
            },
            Choice {
                link: "link5".to_string(),
                name: "choice5".to_string(),
            },
            Choice {
                link: "link6".to_string(),
                name: "choice6".to_string(),
            },
        ];

        let line = "1,2,3".to_string();
        assert_eq!(
            parse_input(line, &choices),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string(),
            ]
        );

        let line = "1-5".to_string();
        assert_eq!(
            parse_input(line, &choices),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string(),
                "link4".to_string(),
                "link5".to_string(),
            ]
        );

        let line = "1-3, 6".to_string();
        assert_eq!(
            parse_input(line, &choices),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string(),
                "link6".to_string(),
            ]
        );

        let line = "1-".to_string();
        assert_eq!(
            parse_input(line, &choices),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string(),
                "link4".to_string(),
                "link5".to_string(),
                "link6".to_string(),
            ]
        );
        let line = "".to_string();
        assert_eq!(
            parse_input(line, &choices),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string(),
                "link4".to_string(),
                "link5".to_string(),
                "link6".to_string(),
            ]
        );

        let line = "1-2, 4-6".to_string();
        assert_eq!(
            parse_input(line, &choices),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link4".to_string(),
                "link5".to_string(),
                "link6".to_string(),
            ]
        );
    }
}
