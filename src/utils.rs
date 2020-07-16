use anyhow::{bail, Result};
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use regex::Regex;

use std::io::prelude::*;

pub const REGEX_VALUE: &str = "_{}";

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

pub fn to_title_case(s: &str) -> String {
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

pub fn instance_multi_bars() -> (MultiProgress, ProgressStyle) {
    let multi = MultiProgress::new();

    // for flickering bar bug (https://github.com/mitsuhiko/indicatif/issues/143)
    multi.set_move_cursor(cfg!(windows));

    (multi, ProgressStyle::default_bar().template("{spinner:.green} [{elapsed}] [{bar:35.cyan/blue}] {bytes}/{total_bytes} ({eta}) {wide_msg}").progress_chars("#>-"))
}

pub fn instance_bar(style: &ProgressStyle) -> ProgressBar {
    let pb = ProgressBar::new(0);
    pb.set_style(style.clone());

    pb
}

pub fn prompt_choices(choices: Vec<(&str, String)>) -> Result<String> {
    Ok(match choices.len() {
        0 => bail!("No match found"),
        1 => choices[0].0.to_string(),
        _ => {
            println!("Found {} matches", choices.len());
            for i in 0..choices.len() {
                println!("[{}] {}", i + 1, choices[i].1);
            }
            print!("\nEnter a number [default=1]: ");
            std::io::stdout().flush()?;

            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let value: usize = line.trim().parse().unwrap_or(1);

            choices[value - 1].0.to_string()
        }
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_info() -> Result<()> {
        let url = "http://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let res: RegInfo = extract_info(url)?;

        assert_eq!(res.name, "Anime Name");
        assert_eq!(res.num, 15);
        assert_eq!(
            res.raw,
            "http://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4"
        );

        Ok(())
    }
}
