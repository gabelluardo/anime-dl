use std::process::exit;

use anyhow::{Result, ensure};

use super::input::{Command, get_command, get_selection};
use super::table::{build_episodes_table, build_table, print_prompt, print_title};
use crate::{
    anilist::WatchingAnime,
    anime::{Anime, EpisodeId},
    error::TuiError,
    range::Range,
};

/// Selects from a list of watching anime
pub fn select_from_watching(series: &[WatchingAnime]) -> Result<Vec<&WatchingAnime>> {
    let rows = {
        let mut rows = Vec::new();
        for (i, c) in series.iter().enumerate() {
            let watched = match c.watched() {
                0 => "•".to_string(),
                n => n.to_string(),
            };
            rows.push(vec![(i + 1).to_string(), c.title().to_string(), watched]);
        }
        rows
    };

    let table = build_table(vec!["Index", "Name", "To See"], rows);

    print_title("You are watching these series");
    println!("{table}");
    print_prompt("Make your selection (eg: 1 2 3 or 1-3) [<u> for unwatched, <q> for exit]");

    let series: Vec<_> = match get_command()? {
        Command::Default(input) => get_selection(&input, 1, series.len())?
            .iter()
            .filter_map(|i| series.get(usize::from(*i) - 1))
            .collect(),
        Command::Unwatched => series.iter().filter(|s| s.watched() > 0).collect(),
        Command::Quit => exit(0),
    };
    println!();

    ensure!(!series.is_empty(), TuiError::InvalidInput);

    Ok(series)
}

/// Selects from a list of anime series
pub fn select_series(series: &mut Vec<Anime>) -> Result<()> {
    let mut rows = Vec::new();
    for (i, c) in series.iter().enumerate() {
        rows.push(vec![(i + 1).to_string(), c.name().to_string()]);
    }

    let table = build_table(vec!["Index", "Name"], rows);

    println!("{table}");
    print_prompt("Make your selection (eg: 1 2 3 or 1-3) [<enter> for all, <q> for exit]");

    match get_command()? {
        Command::Default(input) => {
            *series = get_selection(&input, 1, series.len())?
                .iter()
                .filter_map(|i| series.get(usize::from(*i) - 1).cloned())
                .collect()
        }
        _ => exit(0),
    };
    println!();

    Ok(())
}

/// Selects episodes from an anime
pub fn select_episodes(anime: &Anime) -> Result<Vec<String>> {
    let last_watched = anime.last_watched();
    let mut next_to_watch = None;
    let mut rows = Vec::new();

    match anime.range() {
        Some(Range { start, end }) => {
            for i in Range::new(EpisodeId(0), end) {
                let index = start + i;
                let watched = last_watched.is_some_and(|l| l > i);

                if next_to_watch.is_none() && !watched {
                    // rows.len() equals i at this point (before pushing the current row)
                    // We need i+1 because episode numbering starts at 1
                    next_to_watch = Some(rows.len() + 1)
                }

                rows.push(vec![index.to_string(), icon(last_watched, i.into())]);
            }
        }
        _ => rows.push(vec![1.to_string(), icon(last_watched, 0)]),
    }

    let table = build_episodes_table(vec!["Episode", "Seen"], rows, next_to_watch);

    print_title(anime.name());
    println!("{table}");
    print_prompt("Make your selection (eg: 1 2 3 or 1-3) [<u> for unwatched, <q> for exit]");

    let episodes = match get_command()? {
        Command::Default(input) => {
            let index_start = anime.next_episode().into();
            let content_len = anime.last_episode().into();

            let selection = get_selection(&input, index_start, content_len)?;

            anime.select_from_slice(&selection)
        }

        Command::Unwatched => {
            let index = next_to_watch.unwrap_or(anime.next_episode().into());

            anime.select_from_index(index.into())
        }

        Command::Quit => exit(0),
    };
    println!();

    Ok(episodes)
}

/// Returns the icon for an episode based on whether it has been watched.
fn icon(last: Option<EpisodeId>, index: u32) -> String {
    if last.is_some_and(|i| i > index.into()) {
        "✔".to_string()
    } else {
        "✗".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case(Some(EpisodeId(5)), 3, "✔"; "watched episode")]
    #[test_case(Some(EpisodeId(5)), 5, "✗"; "current episode not watched")]
    #[test_case(Some(EpisodeId(5)), 10, "✗"; "future episode")]
    #[test_case(None, 0, "✗"; "no last watched")]
    #[test_case(Some(EpisodeId(1)), 0, "✔"; "first episode watched")]
    #[test_case(Some(EpisodeId(0)), 0, "✗"; "zero last watched")]
    #[test]
    fn test_icon(last: Option<EpisodeId>, index: u32, expected: &str) {
        assert_eq!(icon(last, index), expected);
    }
}
