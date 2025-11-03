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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_manager_creation() {
        let manager = ProgressManager::new();
        // Just verify we can create a progress manager
        // We can't easily test the actual progress bar without a TTY
        let pb = manager.add_bar();
        assert_eq!(pb.position(), 0);
    }

    #[test]
    fn test_progress_manager_default() {
        let manager = ProgressManager::default();
        let pb = manager.add_bar();
        assert_eq!(pb.position(), 0);
    }
}
