use anyhow::{bail, Result};
use colored::Colorize;
use regex::Regex;

pub const REGEX_VALUE: &str = "_{}";
pub const CHUNK_SIZE: usize = 1024 * 1024; // 1024^2 = 1MB

pub struct RegInfo {
    pub name: String,
    pub raw: String,
    pub num: u32,
}

pub fn extract_info(string: &str) -> Result<RegInfo> {
    let reg_num = find_first_match(string, r"_\d{2,}")?;
    let reg_name = find_first_match(string, r"\w+[^/]\w+_")?;

    let res: Vec<&str> = reg_name.split("_").collect();
    let name = to_title_case(res[0]);

    let raw = string.replace(reg_num.as_str(), REGEX_VALUE);
    let num: u32 = reg_num.replace("_", "").parse()?;

    Ok(RegInfo { name, raw, num })
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
        if c.is_ascii_alphanumeric() || c.eq_ignore_ascii_case(&'-') {
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

pub fn _format_wrn(s: &str) -> colored::ColoredString {
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
