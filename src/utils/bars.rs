use std::ops::Deref;

pub use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct Bars(MultiProgress);

impl Bars {
    pub fn new() -> Self {
        Self(instance_multi_bars())
    }

    pub fn add_bar(&self) -> ProgressBar {
        self.add(instance_bar())
    }
}

impl Deref for Bars {
    type Target = MultiProgress;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn instance_style() -> ProgressStyle {
    let style = ProgressStyle::default_bar().template("{spinner:.green} [{elapsed:.magenta}] [{bar:20.cyan/blue}] {binary_bytes_per_sec} {bytes:.cyan}/{total_bytes:.blue} ({eta:.magenta}) {msg:.green}").unwrap();

    style.progress_chars("#>-")
}

fn instance_multi_bars() -> MultiProgress {
    let multi = MultiProgress::new();

    // NOTE: fix for flickering bar bug on windows (https://github.com/mitsuhiko/indicatif/issues/143)
    multi.set_move_cursor(cfg!(windows));
    multi
}

fn instance_bar() -> ProgressBar {
    let pb = ProgressBar::new(0);
    pb.set_style(instance_style());
    pb
}
