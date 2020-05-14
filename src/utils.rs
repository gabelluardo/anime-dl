use anyhow::{bail, Result};
use colored::Colorize;
use regex::Regex;

pub const REGEX_VALUE: &str = "_{}";
pub const CHUNK_SIZE: usize = 1024 * 1024; // 1024^2 = 1MB

pub fn extract(url: &str) -> Result<(String, u32)> {
    let res = find_first_match(url, r"_\d{2,}")?;

    let url = url.replace(res.as_str(), REGEX_VALUE);
    let last: u32 = res.replace("_", "").parse()?;

    Ok((url, last))
}

pub fn extract_name(url: &str) -> Result<String> {
    let res = find_first_match(url, r"\w+_")?;
    let res: Vec<&str> = res.split("_").collect();

    let name = to_title_case(res[0]);

    Ok(name)
}

pub fn fix_num_episode(num: u32) -> String {
    format!("_{:02}", num)
}

pub fn find_first_match(url: &str, matcher: &str) -> Result<String> {
    let re = Regex::new(matcher)?;
    let cap = match re.captures_iter(&url).last() {
        Some(c) => c,
        None => bail!("Unable to parse `{}`", url),
    };
    let res = &cap[0];

    Ok(res.to_string())
}

fn to_title_case(s: &str) -> String {
    let mut res = String::new();

    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() || c.is_numeric() {
                res.push(' ');
            }
            res.push(c);
        }
    }

    res.trim().to_string()
}

pub fn format_err(s: anyhow::Error) -> colored::ColoredString {
    format!("[ERROR] {}", s).red()
}

pub fn format_wrn(s: &str) -> colored::ColoredString {
    format!("[WARNING] {}", s).yellow()
}

macro_rules! unwrap_err {
    ($x:expr) => {
        match $x {
            Ok(item) => item,
            Err(err) => {
                eprintln!("{}", $crate::utils::format_err(err));
                return;
            }
        }
    };
}
