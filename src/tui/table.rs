use owo_colors::OwoColorize;
use tabled::{
    builder::Builder,
    settings::{
        Alignment, Modify, Style,
        object::{Columns, Rows, Segment},
        themes::Colorization,
    },
};

use super::style::*;

/// Builds a table with consistent styling for watching anime
pub fn build_watching_table(headers: Vec<&str>, rows: Vec<Vec<String>>) -> String {
    let mut builder = Builder::default();
    builder.push_record(headers);
    for row in rows {
        builder.push_record(row);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Colorization::columns(TABLE_COLORS_WATCHING))
        .with(Modify::new(Rows::first()).with(TABLE_HEADER_COLOR))
        .with(Modify::new(Columns::first()).with(Alignment::center()))
        .with(Modify::new(Columns::last()).with(Alignment::center()));

    table.to_string()
}

/// Builds a table with consistent styling for series selection
pub fn build_series_table(headers: Vec<&str>, rows: Vec<Vec<String>>) -> String {
    let mut builder = Builder::default();
    builder.push_record(headers);
    for row in rows {
        builder.push_record(row);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Colorization::columns(TABLE_COLORS_SERIES))
        .with(Modify::new(Rows::first()).with(TABLE_HEADER_COLOR))
        .with(Modify::new(Columns::first()).with(Alignment::center()));

    table.to_string()
}

/// Builds a table for episode selection with optional highlighting
pub fn build_episodes_table(
    headers: Vec<&str>,
    rows: Vec<Vec<String>>,
    highlight_row: Option<usize>,
) -> String {
    let mut builder = Builder::default();
    builder.push_record(headers);
    for row in rows {
        builder.push_record(row);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Colorization::columns(TABLE_COLORS_EPISODES))
        .with(Modify::new(Rows::first()).with(TABLE_HEADER_COLOR))
        .with(Modify::new(Segment::all()).with(Alignment::center()));

    if let Some(index) = highlight_row {
        use tabled::settings::Color;
        table.with(Colorization::exact(
            [Color::FG_BLACK | Color::BG_WHITE],
            Rows::one(index),
        ));
    }

    table.to_string()
}

/// Prints a selection prompt with consistent formatting
pub fn print_prompt(instructions: &str) {
    println!("\n{} {}", "::".red(), instructions.bold());
}

/// Prints a title header with consistent formatting
pub fn print_title(title: &str) {
    println!("{}\n", title.cyan().bold());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_watching_table() {
        let headers = vec!["Index", "Name", "Behind"];
        let rows = vec![
            vec!["1".to_string(), "Anime 1".to_string(), "3".to_string()],
            vec!["2".to_string(), "Anime 2".to_string(), "0".to_string()],
        ];

        let table = build_watching_table(headers, rows);

        // Verify table contains expected data
        assert!(table.contains("Index"));
        assert!(table.contains("Name"));
        assert!(table.contains("Behind"));
        assert!(table.contains("Anime 1"));
        assert!(table.contains("Anime 2"));
        assert!(table.contains("3"));
    }

    #[test]
    fn test_build_series_table() {
        let headers = vec!["Index", "Name"];
        let rows = vec![
            vec!["1".to_string(), "Series 1".to_string()],
            vec!["2".to_string(), "Series 2".to_string()],
        ];

        let table = build_series_table(headers, rows);

        // Verify table contains expected data
        assert!(table.contains("Index"));
        assert!(table.contains("Name"));
        assert!(table.contains("Series 1"));
        assert!(table.contains("Series 2"));
    }

    #[test]
    fn test_build_episodes_table_no_highlight() {
        let headers = vec!["Episode", "Seen"];
        let rows = vec![
            vec!["1".to_string(), "✔".to_string()],
            vec!["2".to_string(), "✗".to_string()],
        ];

        let table = build_episodes_table(headers, rows, None);

        // Verify table contains expected data
        assert!(table.contains("Episode"));
        assert!(table.contains("Seen"));
        assert!(table.contains("1"));
        assert!(table.contains("2"));
    }

    #[test]
    fn test_build_episodes_table_with_highlight() {
        let headers = vec!["Episode", "Seen"];
        let rows = vec![
            vec!["1".to_string(), "✔".to_string()],
            vec!["2".to_string(), "✗".to_string()],
        ];

        let table = build_episodes_table(headers, rows, Some(2));

        // Verify table contains expected data
        assert!(table.contains("Episode"));
        assert!(table.contains("Seen"));
        assert!(table.contains("1"));
        assert!(table.contains("2"));
    }
}
