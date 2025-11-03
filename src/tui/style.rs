use tabled::settings::Color;

/// Color scheme for table columns
pub const TABLE_COLORS_WATCHING: [Color; 3] =
    [Color::FG_MAGENTA, Color::FG_GREEN, Color::FG_BRIGHT_BLUE];

pub const TABLE_COLORS_SERIES: [Color; 2] = [Color::FG_MAGENTA, Color::FG_GREEN];

pub const TABLE_COLORS_EPISODES: [Color; 2] = [Color::FG_MAGENTA, Color::FG_GREEN];

pub const TABLE_HEADER_COLOR: Color = Color::FG_WHITE;

/// Color scheme and template for progress bar
pub const PROGRESS_BAR_TEMPLATE: &str = "{spinner:.green} [{elapsed:.magenta}] [{bar:20.cyan/blue}] {binary_bytes_per_sec} {bytes:.cyan}/{total_bytes:.blue} ({eta:.magenta}) {msg:.green}";
