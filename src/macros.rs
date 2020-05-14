use colored::Colorize;

pub fn format_err(s: anyhow::Error) -> colored::ColoredString {
    format!("[ERROR] {}", s).red()
}

macro_rules! unwrap_err {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => {
                eprintln!("{}", $crate::macros::format_err(err));
                return;
            }
        }
    };
}
