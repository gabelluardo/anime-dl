use anyhow::{Result, bail, ensure};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use rustyline::{ColorMode, DefaultEditor, config::Configurer};
use tabled::{
    settings::{
        Alignment, Color, Modify,
        object::{Columns, Rows, Segment},
        themes::Colorization,
    },
    {builder::Builder, settings::Style},
};

use crate::anilist::WatchingAnime;
use crate::anime::Anime;
use crate::range::Range;

pub struct Tui {
    bars: MultiProgress,
}

impl Tui {
    pub fn new() -> Self {
        let bars = MultiProgress::new();
        // NOTE: fix for flickering bar bug on windows (https://github.com/mitsuhiko/indicatif/issues/143)
        bars.set_move_cursor(cfg!(windows));

        Self { bars }
    }

    pub fn add_bar(&self) -> ProgressBar {
        let style = ProgressStyle::with_template("{spinner:.green} [{elapsed:.magenta}] [{bar:20.cyan/blue}] {binary_bytes_per_sec} {bytes:.cyan}/{total_bytes:.blue} ({eta:.magenta}) {msg:.green}").unwrap();
        let pb = ProgressBar::new(0).with_style(style.progress_chars("#>-"));

        self.bars.add(pb)
    }

    pub fn select_from_watching(series: &[WatchingAnime]) -> Result<Vec<&WatchingAnime>> {
        let mut builder = Builder::default();
        builder.push_record(["Index", "Name", "Episodes Behind"]);
        for (i, c) in series.iter().enumerate() {
            let behind = match c.behind {
                0 => "•".to_string(),
                n => n.to_string(),
            };

            builder.push_record([(i + 1).to_string(), c.title.clone(), behind]);
        }

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
            "Make your selection (eg: 1 2 3 or 1-3) [<u> for unwatched, <q> for exit]".bold()
        );

        let series: Vec<_> = match parse_commands()? {
            TuiCommand::Default(input) => parse_input(&input, 1, series.len())?
                .iter()
                .filter_map(|i| series.get(i - 1))
                .collect(),
            TuiCommand::Unwatched => series.iter().filter(|s| s.behind > 0).collect(),
            TuiCommand::Quit => quit!(),
        };
        println!();

        ensure!(!series.is_empty(), "Invalid input");

        Ok(series)
    }

    pub fn select_series(series: &mut Vec<Anime>) -> Result<()> {
        let mut builder = Builder::default();
        builder.push_record(["Index", "Name"]);
        for (i, c) in series.iter().enumerate() {
            builder.push_record([(i + 1).to_string(), c.name.clone()]);
        }

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

        match parse_commands()? {
            TuiCommand::Default(input) => {
                *series = parse_input(&input, 1, series.len())?
                    .iter()
                    .filter_map(|i| series.get(i - 1).cloned())
                    .collect()
            }
            _ => quit!(),
        };
        println!();

        Ok(())
    }

    pub fn select_episodes(anime: &Anime) -> Result<Vec<String>> {
        pub fn icon(last: Option<u32>, index: u32) -> String {
            if last.is_some_and(|i| i > index) {
                "✔".to_string()
            } else {
                "✗".to_string()
            }
        }

        let mut next_to_watch = None;
        let mut builder = Builder::default();
        builder.push_record(["Episode", "Seen"]);

        match anime.range {
            Some(Range { start, end }) => {
                for i in 0..end {
                    let index = start + i;
                    let watched = anime.last_watched.is_some_and(|l| l > i);

                    if next_to_watch.is_none() && !watched {
                        next_to_watch = Some(builder.count_records())
                    }

                    builder.push_record([index.to_string(), icon(anime.last_watched, i)]);
                }
            }
            _ => builder.push_record([1.to_string(), icon(anime.last_watched, 0)]),
        }

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

        println!("{}\n", anime.name.cyan().bold());
        println!("{}", table);
        println!(
            "\n{} {}",
            "::".red(),
            "Make your selection (eg: 1 2 3 or 1-3) [<u> for unwatched, <q> for exit]".bold()
        );

        let episodes = match parse_commands()? {
            TuiCommand::Default(input) => {
                let selection = parse_input(
                    &input,
                    anime.start as usize,
                    anime.range.unwrap_or_default().end as usize,
                )?;

                anime.select_from_slice(&selection)
            }
            TuiCommand::Unwatched => match next_to_watch {
                Some(index) => anime.select_from_index(index as u32),
                _ => bail!("Invalid input"),
            },
            TuiCommand::Quit => quit!(),
        };
        println!();

        Ok(episodes)
    }

    #[cfg(feature = "anilist")]
    pub fn get_token(url: &str) -> Result<String> {
        let oauth = "Anilist Oauth".cyan().bold().to_string();
        let action = "Authenticate to:".green().to_string();
        let url = url.magenta().bold().to_string();
        let input = ":: ".red().to_string() + &"Paste token here:".bold().to_string();
        let text = oauth + "\n\n" + &action + " " + &url + "\n\n" + &input;
        println!("{text}");

        let res = match parse_commands()? {
            TuiCommand::Default(line) => line,
            _ => quit!(),
        };

        Ok(res)
    }
}

enum TuiCommand {
    Quit,
    Unwatched,
    Default(String),
}

fn parse_commands() -> Result<TuiCommand> {
    let mut rl = DefaultEditor::new()?;
    rl.set_color_mode(ColorMode::Enabled);
    let prompt = "~❯ ".red().to_string();
    let cmd = match rl.readline(&prompt).map(|line| line.trim().to_owned()) {
        Ok(line) if line.len() == 1 && line.contains(['q', 'Q']) => TuiCommand::Quit,
        Ok(line) if line.len() == 1 && line.contains(['u', 'U']) => TuiCommand::Unwatched,
        Ok(line) => TuiCommand::Default(line),
        Err(err) => bail!(err),
    };

    Ok(cmd)
}

fn parse_input(line: &str, index_start: usize, content_len: usize) -> Result<Vec<usize>> {
    let mut selected = vec![];
    let selection = line
        .split_terminator(&[' ', ','])
        .filter(|s| !s.is_empty())
        .map(|s| s.trim());

    for s in selection {
        if let Ok(num) = s.parse::<usize>() {
            selected.push(num)
        } else if let Ok(range) = Range::parse(s, Some(content_len + index_start - 1)) {
            selected.extend(range)
        } else {
            bail!("Invalid input")
        }
    }

    selected.sort_unstable();
    selected.dedup();

    Ok(selected)
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

        let input = "1,2,3";
        let res = parse_input(input, 1, urls.len()).unwrap();
        assert_eq!(res, vec![1, 2, 3,]);

        let input = "1-5";
        let res = parse_input(input, 1, urls.len()).unwrap();
        assert_eq!(res, vec![1, 2, 3, 4, 5]);

        let input = "1-3, 6";
        let res = parse_input(input, 1, urls.len()).unwrap();
        assert_eq!(res, vec![1, 2, 3, 6]);

        let input = "1-";
        let res = parse_input(input, 1, urls.len()).unwrap();
        assert_eq!(res, vec![1, 2, 3, 4, 5, 6]);

        let input = "";
        let res = parse_input(input, 1, urls.len()).unwrap();
        assert!(res.is_empty());

        let input = "1-2, 4-6";
        let res = parse_input(input, 1, urls.len()).unwrap();
        assert_eq!(res, vec![1, 2, 4, 5, 6]);
    }
}
