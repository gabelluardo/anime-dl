use owo_colors::OwoColorize;
use tabled::{
    Table,
    builder::Builder,
    settings::{
        Alignment, Color, Modify, Style,
        object::{Columns, Rows, Segment},
        themes::Colorization,
    },
};

const HEADER_COLOR: Color = Color::FG_WHITE;
const TABLE_COLORS: [Color; 3] = [Color::FG_MAGENTA, Color::FG_GREEN, Color::FG_BRIGHT_BLUE];

/// Builds a table with consistent styling for watching anime
pub fn build_table(headers: Vec<&str>, rows: Vec<Vec<String>>) -> String {
    let mut table = new_table(headers, rows);
    table
        .with(Style::rounded())
        .with(Colorization::columns(TABLE_COLORS))
        .with(Modify::new(Rows::first()).with(HEADER_COLOR))
        .with(Modify::new(Columns::first()).with(Alignment::center()))
        .with(Modify::new(Columns::one(2)).with(Alignment::center()));

    table.to_string()
}

/// Builds a table for episode selection with optional highlighting
pub fn build_episodes_table(
    headers: Vec<&str>,
    rows: Vec<Vec<String>>,
    highlighted_row: Option<usize>,
) -> String {
    let mut table = new_table(headers, rows);
    table
        .with(Style::rounded())
        .with(Colorization::columns(TABLE_COLORS))
        .with(Modify::new(Rows::first()).with(HEADER_COLOR))
        .with(Modify::new(Segment::all()).with(Alignment::center()));

    if let Some(index) = highlighted_row {
        let colors = [Color::FG_BLACK | Color::BG_WHITE];
        table.with(Colorization::exact(colors, Rows::one(index)));
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

fn new_table(headers: Vec<&str>, rows: Vec<Vec<String>>) -> Table {
    let mut builder = Builder::default();
    builder.push_record(headers);
    for row in rows {
        builder.push_record(row);
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case(
        vec!["Index", "Name", "Behind"],
        vec![
            vec!["1".to_string(), "Anime 1".to_string(), "3".to_string()],
            vec!["2".to_string(), "Anime 2".to_string(), "0".to_string()],
        ],
        vec!["Index", "Name", "Behind", "Anime 1", "Anime 2", "3"];
        "watching table"
    )]
    #[test_case(
        vec!["Index", "Name"],
        vec![
            vec!["1".to_string(), "Series 1".to_string()],
            vec!["2".to_string(), "Series 2".to_string()],
        ],
        vec!["Index", "Name", "Series 1", "Series 2"];
        "series table"
    )]
    #[test]
    fn test_build_table(headers: Vec<&str>, rows: Vec<Vec<String>>, expected: Vec<&str>) {
        let table = build_table(headers, rows);

        for value in expected {
            assert!(table.contains(value));
        }
    }

    #[test_case(None; "without highlight")]
    #[test_case(Some(2); "with highlight")]
    #[test]
    fn test_build_episodes_table(highlighted_row: Option<usize>) {
        let headers = vec!["Episode", "Seen"];
        let rows = vec![
            vec!["1".to_string(), "✔".to_string()],
            vec!["2".to_string(), "✗".to_string()],
        ];

        let table = build_episodes_table(headers, rows, highlighted_row);

        for value in ["Episode", "Seen", "1", "2"] {
            assert!(table.contains(value));
        }
    }

    #[test_case("test instructions"; "prompt")]
    #[test_case("another prompt"; "another prompt")]
    #[test]
    fn test_print_prompt(instructions: &str) {
        print_prompt(instructions);
    }

    #[test_case("test title"; "title")]
    #[test_case("another title"; "another title")]
    #[test]
    fn test_print_title(title: &str) {
        print_title(title);
    }

    #[test_case(vec!["A", "B"], vec!["A", "B"]; "empty rows")]
    #[test_case(vec!["Index", "Name"], vec!["Index", "Name"]; "two headers")]
    #[test]
    fn test_build_table_empty(headers: Vec<&str>, expected: Vec<&str>) {
        let table = build_table(headers, vec![]);
        for value in expected {
            assert!(table.contains(value));
        }
    }

    #[test_case(None; "empty no highlight")]
    #[test_case(Some(0); "empty highlight first")]
    #[test]
    fn test_build_episodes_table_empty(highlighted_row: Option<usize>) {
        let table = build_episodes_table(vec!["Episode", "Seen"], vec![], highlighted_row);
        assert!(table.contains("Episode"));
        assert!(table.contains("Seen"));
    }

    #[test_case(Some(0); "highlight first row")]
    #[test_case(Some(1); "highlight second row")]
    #[test_case(None; "no highlight")]
    #[test_case(Some(2); "highlight third row")]
    #[test]
    fn test_build_episodes_table_highlight_rows(highlighted_row: Option<usize>) {
        let headers = vec!["Episode", "Seen"];
        let rows = vec![
            vec!["1".to_string(), "✔".to_string()],
            vec!["2".to_string(), "✗".to_string()],
        ];

        let table = build_episodes_table(headers, rows, highlighted_row);
        assert!(table.contains("1"));
        assert!(table.contains("2"));
    }
}
