use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

const BAR_TEMPLATE: &str = "{spinner:.green} [{elapsed:.magenta}] [{bar:20.cyan/blue}] {binary_bytes_per_sec} {bytes:.cyan}/{total_bytes:.blue} ({eta:.magenta}) {msg:.green}";

/// Manages progress bars for downloads
pub struct ProgressManager {
    bars: MultiProgress,
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressManager {
    pub fn new() -> Self {
        let bars = MultiProgress::new();

        // NOTE: fix for flickering bar bug on windows (https://github.com/mitsuhiko/indicatif/issues/143)
        bars.set_move_cursor(cfg!(windows));

        Self { bars }
    }

    pub fn add_bar(&self) -> ProgressBar {
        let style = ProgressStyle::with_template(BAR_TEMPLATE).unwrap();
        let pb = ProgressBar::new(0).with_style(style.progress_chars("#>-"));

        self.bars.add(pb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case(0; "new manager bar at zero")]
    #[test]
    fn test_progress_manager_creation(_dummy: u32) {
        let manager = ProgressManager::new();
        let pb = manager.add_bar();
        assert_eq!(pb.position(), 0);
    }

    #[test_case(0; "default manager bar at zero")]
    #[test]
    fn test_progress_manager_default(_: u32) {
        let manager = ProgressManager::default();
        let pb = manager.add_bar();
        assert_eq!(pb.position(), 0);
    }
}
