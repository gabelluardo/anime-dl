use anyhow::{Result, bail};
use owo_colors::OwoColorize;
use rustyline::{ColorMode, DefaultEditor, config::Configurer};

use crate::{anime::EpisodeId, error::TuiError};

/// Commands that can be parsed from user input
pub enum Command {
    Quit,
    Unwatched,
    Default(String),
}

/// Parses user input commands from the terminal
pub fn get_command() -> Result<Command> {
    let mut rl = DefaultEditor::new()?;
    rl.set_color_mode(ColorMode::Enabled);
    let prompt = "~❯ ".red().to_string();
    let cmd = match rl.readline(&prompt).map(|line| line.trim().to_owned()) {
        Ok(line) if line.len() == 1 && line.contains(['q', 'Q']) => Command::Quit,
        Ok(line) if line.len() == 1 && line.contains(['u', 'U']) => Command::Unwatched,
        Ok(line) => Command::Default(line),
        Err(_) => bail!(TuiError::InvalidInput),
    };

    Ok(cmd)
}

/// Parses a selection string into a list of indices
///
/// Supports formats like:
/// - "1,2,3" - individual selections
/// - "1-3" - range selection
/// - "1-" - open-ended range (uses content_len)
pub fn get_selection(line: &str, index_start: usize, content_len: usize) -> Result<Vec<EpisodeId>> {
    use crate::range::Range;

    let mut selected = Vec::new();
    let selection: Vec<_> = line
        .split_terminator(&[' ', ','])
        .filter(|s| !s.is_empty())
        .map(|s| s.trim())
        .collect();

    for s in selection {
        if let Ok(num) = s.parse::<usize>() {
            selected.push(num.into())
        } else if let Ok(range) =
            Range::<EpisodeId>::parse(s, Some((content_len + index_start - 1).into()))
        {
            selected.extend(range)
        } else {
            bail!(TuiError::InvalidInput)
        }
    }

    selected.sort_unstable();
    selected.dedup();

    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case("1,2,3", vec![1, 2, 3]; "comma separated")]
    #[test_case("1-5", vec![1, 2, 3, 4, 5]; "closed range")]
    #[test_case("1-3, 6", vec![1, 2, 3, 6]; "range and single")]
    #[test_case("1-", vec![1, 2, 3, 4, 5, 6]; "open ended range")]
    #[test_case("", Vec::new(); "empty input")]
    #[test_case("1-2, 4-6", vec![1, 2, 4, 5, 6]; "multiple ranges")]
    #[test]
    fn test_parse_input(input: &str, expected: Vec<usize>) {
        let expected: Vec<_> = expected.into_iter().map(|n| n.into()).collect();
        let content_len = 6;
        let res = get_selection(input, 1, content_len).unwrap();
        assert_eq!(res, expected);
    }
}
