pub use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

fn instance_style() -> ProgressStyle {
    ProgressStyle::default_bar().template("{spinner:.green} [{elapsed}] [{bar:20.cyan/blue}] {bytes_per_sec} {bytes}/{total_bytes} ({eta}) {wide_msg}").progress_chars("#>-")
}

pub fn instance_multi_bars() -> MultiProgress {
    let multi = MultiProgress::new();
    // for flickering bar bug on windows (https://github.com/mitsuhiko/indicatif/issues/143)
    multi.set_move_cursor(cfg!(windows));
    multi
}

pub fn instance_bar() -> ProgressBar {
    let pb = ProgressBar::new(0);
    pb.set_style(instance_style());
    pb
}
