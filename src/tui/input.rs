use anyhow::{Result, bail};
use owo_colors::OwoColorize;
use rustyline::{ColorMode, DefaultEditor, config::Configurer};

/// Commands that can be parsed from user input
pub enum Command {
    Quit,
    Unwatched,
    Default(String),
}

/// Parses user input commands from the terminal
pub fn parse_commands() -> Result<Command> {
    let mut rl = DefaultEditor::new()?;
    rl.set_color_mode(ColorMode::Enabled);
    let prompt = "~❯ ".red().to_string();
    let cmd = match rl.readline(&prompt).map(|line| line.trim().to_owned()) {
        Ok(line) if line.len() == 1 && line.contains(['q', 'Q']) => Command::Quit,
        Ok(line) if line.len() == 1 && line.contains(['u', 'U']) => Command::Unwatched,
        Ok(line) => Command::Default(line),
        Err(err) => bail!(err),
    };

    Ok(cmd)
}

/// Parses a selection string into a list of indices
///
/// Supports formats like:
/// - "1,2,3" - individual selections
/// - "1-3" - range selection
/// - "1-" - open-ended range (uses content_len)
pub fn parse_input(line: &str, index_start: usize, content_len: usize) -> Result<Vec<usize>> {
    use crate::range::Range;

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
