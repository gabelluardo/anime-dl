use anyhow::{Result, bail};
use owo_colors::OwoColorize;
use rustyline::{ColorMode, DefaultEditor, config::Configurer, error::ReadlineError};

use crate::{anime::EpisodeId, error::TuiError};

/// Commands that can be parsed from user input
#[derive(Debug)]
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
        Ok(line) if line.eq_ignore_ascii_case("q") => Command::Quit,
        Ok(line) if line.eq_ignore_ascii_case("u") => Command::Unwatched,
        Ok(line) => Command::Default(line),
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => Command::Quit,
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

    // content_len == 0 would cause underflow in `max` and means there is
    // nothing to select anyway.
    if content_len == 0 {
        bail!(TuiError::InvalidInput);
    }

    let max = index_start + content_len - 1;
    let mut selected = Vec::new();
    let selection: Vec<_> = line.split(&[' ', ',']).filter(|s| !s.is_empty()).collect();

    for s in selection {
        if let Ok(num) = s.parse::<usize>() {
            selected.push(num.into())
        } else if let Ok(range) = Range::<EpisodeId>::parse(s, Some(max.into())) {
            selected.extend(range)
        } else {
            bail!(TuiError::InvalidInput)
        }
    }

    // Validate every index falls within [index_start, max]. Range::parse clamps
    // the high end, but neither it nor single-value parsing reject out-of-bounds
    // starts or values.
    for &n in &selected {
        let n: usize = n.into();
        if n < index_start || n > max {
            bail!(TuiError::InvalidInput);
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
    #[test_case("3", vec![3]; "single value not expanded")]
    #[test]
    fn test_parse_input(input: &str, expected: Vec<usize>) {
        let expected: Vec<_> = expected.into_iter().map(|n| n.into()).collect();
        let content_len = 6;
        let res = get_selection(input, 1, content_len).unwrap();
        assert_eq!(res, expected);
    }

    #[test_case("0"; "below index_start")]
    #[test_case("7"; "above max")]
    #[test_case("99"; "far above max")]
    #[test_case("0-3"; "range below index_start")]
    #[test_case("abc"; "non numeric")]
    #[test_case("1-3,abc"; "mixed valid and invalid")]
    #[test]
    fn test_invalid_input(input: &str) {
        let res = get_selection(input, 1, 6);
        assert!(res.is_err(), "expected error for input: {input}");
    }

    #[test_case("1-7", 1, 6, vec![1, 2, 3, 4, 5, 6]; "range clamped to max")]
    #[test_case("5-7", 5, 3, vec![5, 6, 7]; "non default index start")]
    #[test_case("0-2", 0, 3, vec![0, 1, 2]; "zero indexed episodes")]
    #[test_case("0", 0, 6, vec![0]; "single episode zero")]
    #[test]
    fn test_valid_selection(
        input: &str,
        index_start: usize,
        content_len: usize,
        expected: Vec<usize>,
    ) {
        let expected: Vec<_> = expected.into_iter().map(|n| n.into()).collect();
        let res = get_selection(input, index_start, content_len).unwrap();
        assert_eq!(res, expected);
    }

    #[test_case("1", 1, 0; "content len zero")]
    #[test_case("8", 5, 3; "above max with non default start")]
    #[test_case("4", 5, 3; "below index_start with non default start")]
    #[test_case("3", 0, 3; "above max with zero index start")]
    #[test]
    fn test_invalid_selection(input: &str, index_start: usize, content_len: usize) {
        assert!(get_selection(input, index_start, content_len).is_err());
    }
}
