use std::process::exit;

use anyhow::{Result, ensure};

use crate::anilist::WatchingAnime;
use crate::anime::Anime;
use crate::range::Range;

use super::input::{Command, parse_commands, parse_input};
use super::table::{
    build_episodes_table, build_series_table, build_watching_table, print_prompt, print_title,
};

/// Selects from a list of watching anime
pub fn select_from_watching(series: &[WatchingAnime]) -> Result<Vec<&WatchingAnime>> {
    let mut rows = vec![];
    for (i, c) in series.iter().enumerate() {
        let watched = match c.watched() {
            0 => "•".to_string(),
            n => n.to_string(),
        };
        rows.push(vec![(i + 1).to_string(), c.title(), watched]);
    }

    let table = build_watching_table(vec!["Index", "Name", "Episodes Behind"], rows);

    print_title("You are watching these series");
    println!("{table}");
    print_prompt("Make your selection (eg: 1 2 3 or 1-3) [<u> for unwatched, <q> for exit]");

    let series: Vec<_> = match parse_commands()? {
        Command::Default(input) => parse_input(&input, 1, series.len())?
            .iter()
            .filter_map(|i| series.get(i - 1))
            .collect(),
        Command::Unwatched => series.iter().filter(|s| s.watched() > 0).collect(),
        Command::Quit => exit(0),
    };
    println!();

    ensure!(!series.is_empty(), "Invalid input");

    Ok(series)
}

/// Selects from a list of anime series
pub fn select_series(series: &mut Vec<Anime>) -> Result<()> {
    let mut rows = vec![];
    for (i, c) in series.iter().enumerate() {
        rows.push(vec![(i + 1).to_string(), c.name.clone()]);
    }

    let table = build_series_table(vec!["Index", "Name"], rows);

    println!("{table}");
    print_prompt("Make your selection (eg: 1 2 3 or 1-3) [<enter> for all, <q> for exit]");

    match parse_commands()? {
        Command::Default(input) => {
            *series = parse_input(&input, 1, series.len())?
                .iter()
                .filter_map(|i| series.get(i - 1).cloned())
                .collect()
        }
        _ => exit(0),
    };
    println!();

    Ok(())
}

/// Selects episodes from an anime
pub fn select_episodes(anime: &Anime) -> Result<Vec<String>> {
    fn icon(last: Option<i64>, index: u32) -> String {
        if last.is_some_and(|i| i > index.into()) {
            "✔".to_string()
        } else {
            "✗".to_string()
        }
    }

    let mut next_to_watch = None;
    let mut rows = vec![];

    match anime.range {
        Some(Range { start, end }) => {
            for i in 0..end {
                let index = start + i;
                let watched = anime.last_watched.is_some_and(|l| l > i.into());

                if next_to_watch.is_none() && !watched {
                    // rows.len() equals i at this point (before pushing the current row)
                    // We need i+1 because episode numbering starts at 1
                    next_to_watch = Some(rows.len() + 1)
                }

                rows.push(vec![index.to_string(), icon(anime.last_watched, i)]);
            }
        }
        _ => rows.push(vec![1.to_string(), icon(anime.last_watched, 0)]),
    }

    let table = build_episodes_table(vec!["Episode", "Seen"], rows, next_to_watch);

    print_title(&anime.name);
    println!("{table}");
    print_prompt("Make your selection (eg: 1 2 3 or 1-3) [<u> for unwatched, <q> for exit]");

    let episodes = match parse_commands()? {
        Command::Default(input) => {
            let selection = parse_input(
                &input,
                anime.start as usize,
                anime.range.unwrap_or_default().end as usize,
            )?;

            anime.select_from_slice(&selection)
        }
        Command::Unwatched => match next_to_watch {
            Some(index) => anime.select_from_index(index as u32),
            _ => anyhow::bail!("Invalid input"),
        },
        Command::Quit => exit(0),
    };
    println!();

    Ok(episodes)
}
