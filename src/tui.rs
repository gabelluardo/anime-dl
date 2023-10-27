use std::ops::Deref;

use anyhow::{bail, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
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

use crate::anilist::WatchingAnime;
use crate::anime::{Anime, AnimeInfo};
use crate::errors::{Quit, RemoteError, UserError};
use crate::range::Range;

fn parse_input<T: Clone>(line: &str, content: &[T], index_start: usize) -> Vec<T> {
    let mut selected = vec![];
    let line = line
        .replace([',', '.'], " ")
        .chars()
        .filter(|c| c.is_ascii_digit() || c.is_ascii_whitespace() || *c == '-')
        .collect::<String>();

    for s in line.split_ascii_whitespace().map(|s| s.trim()) {
        if let Ok(num) = s.parse::<usize>() {
            selected.push(num);
        } else if let Ok(range) = Range::<usize>::parse_and_fill(s, content.len()) {
            selected.extend(range.expand())
        }
    }

    selected.sort_unstable();
    selected.dedup();

    if selected.is_empty() {
        content.to_vec()
    } else {
        selected
            .iter()
            .filter_map(|i| content.get(i - index_start).cloned())
            .collect()
    }
}

pub fn watching_choice(series: &[WatchingAnime]) -> Result<Vec<WatchingAnime>> {
    if series.len() == 1 {
        return Ok(series.to_vec());
    }

    let mut builder = Builder::default();
    builder.set_header(["Index", "Name", "Episodes Behind"]);
    series.iter().enumerate().for_each(|(i, c)| {
        let behind = match c.behind {
            0 => "•".to_string(),
            n => n.to_string(),
        };

        builder.push_record([(i + 1).to_string(), c.title.clone(), behind]);
    });

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Colorization::columns([
            Color::FG_MAGENTA,
            Color::FG_GREEN,
            Color::FG_BRIGHT_BLUE,
        ]))
        .with(Modify::new(Rows::first()).with(Color::FG_WHITE))
        .with(Modify::new(Columns::first()).with(Alignment::center()))
        .with(Modify::new(Columns::last()).with(Alignment::center()));

    let str = "You are watching these series".cyan().bold().to_string();
    println!("{str}\n",);
    println!("{}", table);
    println!(
        "\n{} {}",
        "::".red(),
        "Make your selection (eg: 1 2 3 or 1-3) [<enter> for all, <q> for exit]".bold()
    );

    let mut rl = DefaultEditor::new()?;
    rl.set_color_mode(ColorMode::Enabled);
    let prompt = "~❯ ".red().to_string();
    let res = match rl.readline(&prompt) {
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => bail!(Quit),
        Err(_) => bail!(UserError::InvalidInput),
        Ok(line) if line.contains(['q', 'Q']) => bail!(Quit),
        Ok(line) => parse_input(&line, series, 1),
    };
    println!();

    if res.is_empty() {
        bail!(RemoteError::AnimeNotFound);
    }

    Ok(res)
}

pub fn series_choice(series: &[AnimeInfo], search: &str) -> Result<Vec<AnimeInfo>> {
    if series.len() == 1 {
        return Ok(series.to_vec());
    }

    let len = series.len();
    let query = search.replace('+', " ");
    let results = format!("{len} results found for `{query}`");
    println!("{}\n", results.cyan().bold());

    let mut builder = Builder::default();
    builder.set_header(["Index", "Name"]);
    series.iter().enumerate().for_each(|(i, c)| {
        builder.push_record([(i + 1).to_string(), c.name.clone()]);
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

    let mut rl = DefaultEditor::new()?;
    rl.set_color_mode(ColorMode::Enabled);
    let prompt = "~❯ ".red().to_string();
    let res = match rl.readline(&prompt) {
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => bail!(Quit),
        Err(_) => bail!(UserError::InvalidInput),
        Ok(line) if line.contains(['q', 'Q']) => bail!(Quit),
        Ok(line) => parse_input(&line, series, 1),
    };
    println!();

    if res.is_empty() {
        bail!(RemoteError::AnimeNotFound);
    }

    Ok(res)
}

pub fn episodes_choice(anime: &Anime) -> Result<Vec<String>> {
    if anime.episodes.len() == 1 {
        return Ok(vec![anime.info.origin.to_owned()]);
    }

    let mut next_to_watch = None;
    let mut builder = Builder::default();
    builder.set_header(["Episode", "Seen"]);
    anime.episodes.iter().enumerate().for_each(|(i, _)| {
        let index = anime.start + i as u32;
        let watched = Some(i as u32) < anime.last_watched;
        let check = if watched { "✔" } else { "✗" };

        if next_to_watch.is_none() && !watched {
            next_to_watch = Some(builder.count_rows() + 1)
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

    println!("{}\n", anime.info.name.cyan().bold());
    println!("{}", table);
    println!(
        "\n{} {}",
        "::".red(),
        "Make your selection (eg: 1 2 3 or 1-3) [<enter> for all, <q> for exit, <u> for unwatched]"
            .bold()
    );

    let mut rl = DefaultEditor::new()?;
    rl.set_color_mode(ColorMode::Enabled);
    let prompt = "~❯ ".red().to_string();
    let res = match rl.readline(&prompt) {
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => bail!(Quit),
        Err(_) => bail!(UserError::InvalidInput),
        Ok(line) if line.contains(['q', 'Q']) => bail!(Quit),
        Ok(line) if line.contains(['u', 'U']) => {
            if let Some(index) = next_to_watch {
                anime.episodes[index - 1..].to_vec()
            } else {
                bail!(UserError::InvalidInput)
            }
        }
        Ok(line) => parse_input(&line, &anime.episodes, anime.start as usize),
    };
    println!();

    if res.is_empty() {
        bail!(RemoteError::EpisodeNotFound);
    }

    Ok(res)
}

#[cfg(feature = "anilist")]
pub fn get_token(url: &str) -> Result<String> {
    let oauth = "Anilist Oauth".cyan().bold().to_string();
    let action = "Authenticate to:".green().to_string();
    let url = url.magenta().bold().to_string();
    let input = ":: ".red().to_string() + &"Paste token here:".bold().to_string();
    let text = oauth + "\n\n" + &action + " " + &url + "\n\n" + &input;
    println!("{text}");

    let mut rl = DefaultEditor::new()?;
    let prompt = "~❯ ".red().to_string();
    let res = match rl.readline(&prompt) {
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => bail!(Quit),
        Err(_) => bail!(UserError::InvalidInput),
        Ok(line) if line.trim().len() == 1 && line.contains(['q', 'Q']) => bail!(Quit),
        Ok(line) => line.trim().to_string(),
    };

    Ok(res)
}

pub struct Bars(MultiProgress);

impl Deref for Bars {
    type Target = MultiProgress;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Bars {
    pub fn new() -> Self {
        let multi = MultiProgress::new();
        // NOTE: fix for flickering bar bug on windows (https://github.com/mitsuhiko/indicatif/issues/143)
        multi.set_move_cursor(cfg!(windows));

        Self(multi)
    }

    pub fn add_bar(&self) -> ProgressBar {
        let style = ProgressStyle::with_template("{spinner:.green} [{elapsed:.magenta}] [{bar:20.cyan/blue}] {binary_bytes_per_sec} {bytes:.cyan}/{total_bytes:.blue} ({eta:.magenta}) {msg:.green}").unwrap();
        let pb = ProgressBar::new(0).with_style(style.progress_chars("#>-"));

        self.add(pb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_input() {
        let urls: Vec<String> = vec![
            "link1".into(),
            "link2".into(),
            "link3".into(),
            "link4".into(),
            "link5".into(),
            "link6".into(),
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
