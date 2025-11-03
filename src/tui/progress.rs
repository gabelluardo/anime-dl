use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Manages progress bars for downloads
pub struct ProgressManager {
    bars: MultiProgress,
}

impl ProgressManager {
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
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}
