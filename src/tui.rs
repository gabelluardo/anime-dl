use crate::errors::{Quit, RemoteError, UserError};
use crate::range::Range;

use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use rustyline::{config::Configurer, error::ReadlineError, ColorMode, DefaultEditor};
use tabled::{
    settings::{
        object::{Columns, Rows, Segment},
        themes::Colorization,
        Alignment, Color, Modify,
    },
    {builder::Builder, settings::Style},
};

#[derive(Clone)]
pub struct Choice {
    link: String,
    name: String,
    watched: Option<bool>,
}

impl Choice {
    pub fn new(link: &str, name: &str, watched: Option<bool>) -> Self {
        Self {
            watched,
            link: link.to_owned(),
            name: name.to_owned(),
        }
    }
}

fn parse_input(line: &str, choices: &[Choice]) -> Vec<String> {
    let mut selected = vec![];
    let line = line
        .replace([',', '.'], " ")
        .chars()
        .filter(|c| c.is_ascii_digit() || c.is_ascii_whitespace() || *c == '-')
        .collect::<String>();
    let sel = line
        .split_ascii_whitespace()
        .map(|s| s.trim())
        .collect::<Vec<_>>();
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
        0 => choices.iter().map(|c| c.link.clone()).collect::<Vec<_>>(),
        _ => selected
            .iter()
            .filter_map(|i| choices.get(i - 1))
            .map(|c| c.link.clone())
            .collect::<Vec<_>>(),
    }
}

pub fn get_choice(choices: &[Choice], query: Option<String>) -> Result<Vec<String>> {
    match choices.len() {
        0 => bail!(UserError::Choices),
        1 => Ok(vec![choices[0].link.clone()]),
        _ => {
            let len = choices.len();
            let is_anime_list = choices[0].watched.is_none();
            let name = query.map(|n| format!(" for `{n}`")).unwrap_or_default();
            let results = format!("{len} results found{name}");
            println!("{}\n", results.cyan().bold());

            let mut builder = Builder::default();
            if is_anime_list {
                builder.set_header(["Index", "Name"]);
            } else {
                builder.set_header(["Episode", "Seen"]);
            }

            for (i, c) in choices.iter().enumerate() {
                let index = (i + 1).to_string();

                if is_anime_list {
                    builder.push_record([index, c.name.clone()]);
                } else {
                    let check = if c.watched.unwrap() { "✔" } else { "✗" };

                    builder.push_record([index, check.to_string()]);
                }
            }

            let mut table = builder.build();
            table
                .with(Style::rounded())
                .with(Colorization::columns([Color::FG_MAGENTA, Color::FG_GREEN]))
                .with(Modify::new(Rows::first()).with(Color::FG_WHITE));

            if is_anime_list {
                table.with(Modify::new(Columns::first()).with(Alignment::center()));
            } else {
                table.with(Modify::new(Segment::all()).with(Alignment::center()));
            }

            println!("{}", table);
            println!(
                "\n{} {}",
                "::".red(),
                "Make your selection (eg: 1 2 3 or 1-3) [<enter> for all, <q> for exit]".bold()
            );

            let mut rl = DefaultEditor::new().context(UserError::InvalidInput)?;
            rl.set_color_mode(ColorMode::Enabled);
            let prompt = "~❯ ".red().to_string();
            let urls = match rl.readline(&prompt) {
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => bail!(Quit),
                Err(_) => bail!(UserError::InvalidInput),
                Ok(line) if line.contains('q') => bail!(Quit),
                Ok(line) => parse_input(&line, choices),
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
    let oauth = "Anilist Oauth".cyan().bold().to_string();
    let action = "Authenticate to:".green().to_string();
    let url = url.magenta().bold().to_string();
    let input = ":: ".red().to_string() + &"Paste token here:".bold().to_string();
    let text = oauth + "\n\n" + &action + " " + &url + "\n\n" + &input;
    println!("{text}");

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
            Choice::new("link1", "choice1", None),
            Choice::new("link2", "choice2", None),
            Choice::new("link3", "choice3", None),
            Choice::new("link4", "choice4", None),
            Choice::new("link5", "choice5", Some(true)),
            Choice::new("link6", "choice6", Some(true)),
        ];

        let line = "1,2,3";
        assert_eq!(
            parse_input(line, &choices),
            vec!["link1", "link2", "link3",]
        );

        let line = "1-5";
        assert_eq!(
            parse_input(line, &choices),
            vec!["link1", "link2", "link3", "link4", "link5",]
        );

        let line = "1-3, 6";
        assert_eq!(
            parse_input(line, &choices),
            vec!["link1", "link2", "link3", "link6",]
        );

        let line = "1-";
        assert_eq!(
            parse_input(line, &choices),
            vec!["link1", "link2", "link3", "link4", "link5", "link6",]
        );
        let line = "";
        assert_eq!(
            parse_input(line, &choices),
            vec!["link1", "link2", "link3", "link4", "link5", "link6",]
        );

        let line = "1-2, 4-6";
        assert_eq!(
            parse_input(line, &choices),
            vec!["link1", "link2", "link4", "link5", "link6",]
        );
    }
}
