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
}

impl Choice {
    pub fn new(link: &str, name: &str) -> Self {
        Self {
            link: link.to_owned(),
            name: name.to_owned(),
        }
    }
}

fn parse_input(line: &str, urls: &[String], index_start: usize) -> Vec<String> {
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
        } else if let Ok(range) = Range::<usize>::parse_and_fill(s, urls.len()) {
            selected.extend(range.expand())
        }
    }

    selected.sort_unstable();
    selected.dedup();

    if selected.is_empty() {
        urls.to_vec()
    } else {
        selected
            .iter()
            .filter_map(|i| urls.get(i - index_start).cloned())
            .collect::<Vec<_>>()
    }
}

pub fn series_choice(series: &[Choice], query: String) -> Result<Vec<String>> {
    match series.len() {
        0 => bail!(UserError::Choices),
        1 => Ok(vec![series[0].link.to_owned()]),
        _ => {
            let len = series.len();
            let index_start = 1;
            let results = format!("{len} results found for `{query}`");
            println!("{}\n", results.cyan().bold());

            let mut builder = Builder::default();
            builder.set_header(["Index", "Name"]);
            series.iter().enumerate().for_each(|(i, c)| {
                builder.push_record([(i + index_start).to_string(), c.name.clone()]);
            });

            let mut table = builder.build();
            table
                .with(Style::rounded())
                .with(Colorization::columns([Color::FG_MAGENTA, Color::FG_GREEN]))
                .with(Modify::new(Rows::first()).with(Color::FG_WHITE))
                .with(Modify::new(Columns::first()).with(Alignment::center()));

            println!("{}", table);
            println!(
                "\n{} {}",
                "::".red(),
                "Make your selection (eg: 1 2 3 or 1-3) [<enter> for all, <q> for exit]".bold()
            );

            let urls = series.iter().map(|c| c.link.clone()).collect::<Vec<_>>();
            let mut rl = DefaultEditor::new().context(UserError::InvalidInput)?;
            rl.set_color_mode(ColorMode::Enabled);
            let prompt = "~❯ ".red().to_string();
            let res = match rl.readline(&prompt) {
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => bail!(Quit),
                Err(_) => bail!(UserError::InvalidInput),
                Ok(line) if line.contains(['q', 'Q']) => bail!(Quit),
                Ok(line) => parse_input(&line, &urls, index_start),
            };
            println!();

            if res.is_empty() {
                bail!(RemoteError::EpisodeNotFound);
            }

            Ok(res)
        }
    }
}

pub fn episodes_choice(
    episodes: &[String],
    last_watched: Option<u32>,
    name: &str,
    start_range: u32,
) -> Result<Vec<String>> {
    match episodes.len() {
        0 => bail!(UserError::Choices),
        1 => Ok(vec![episodes[0].to_owned()]),
        _ => {
            println!(" {}", name.cyan().bold());

            let mut next_to_watch = None;
            let mut builder = Builder::default();
            builder.set_header(["Episode", "Seen"]);
            episodes.iter().enumerate().for_each(|(i, _)| {
                let index = start_range + i as u32;
                let watched = Some(index) <= last_watched;
                let check = if watched { "✔" } else { "✗" };

                if next_to_watch.is_none() && !watched {
                    next_to_watch = Some(index as usize)
                }

                builder.push_record([index.to_string(), check.to_string()]);
            });

            let mut table = builder.build();

            table
                .with(Style::rounded())
                .with(Colorization::columns([Color::FG_MAGENTA, Color::FG_GREEN]))
                .with(Modify::new(Rows::first()).with(Color::FG_WHITE))
                .with(Modify::new(Segment::all()).with(Alignment::center()));

            if let Some(index) = next_to_watch {
                table.with(Colorization::exact(
                    [Color::FG_BLACK | Color::BG_WHITE],
                    Rows::single(index),
                ));
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
            let res = match rl.readline(&prompt) {
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => bail!(Quit),
                Err(_) => bail!(UserError::InvalidInput),
                Ok(line) if line.contains(['q', 'Q']) => bail!(Quit),
                Ok(line) => parse_input(&line, episodes, start_range as usize),
            };
            println!();

            if res.is_empty() {
                bail!(RemoteError::EpisodeNotFound);
            }

            Ok(res)
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
        let urls = vec![
            "link1".to_string(),
            "link2".to_string(),
            "link3".to_string(),
            "link4".to_string(),
            "link5".to_string(),
            "link6".to_string(),
        ];

        let line = "1,2,3";
        assert_eq!(
            parse_input(line, &urls, 1),
            vec!["link1", "link2", "link3",]
        );

        let line = "1-5";
        assert_eq!(
            parse_input(line, &urls, 1),
            vec!["link1", "link2", "link3", "link4", "link5",]
        );

        let line = "1-3, 6";
        assert_eq!(
            parse_input(line, &urls, 1),
            vec!["link1", "link2", "link3", "link6",]
        );

        let line = "1-";
        assert_eq!(
            parse_input(line, &urls, 1),
            vec!["link1", "link2", "link3", "link4", "link5", "link6",]
        );
        let line = "";
        assert_eq!(
            parse_input(line, &urls, 1),
            vec!["link1", "link2", "link3", "link4", "link5", "link6",]
        );

        let line = "1-2, 4-6";
        assert_eq!(
            parse_input(line, &urls, 1),
            vec!["link1", "link2", "link4", "link5", "link6",]
        );
    }
}
