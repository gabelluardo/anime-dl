use anyhow::{bail, Result};
use regex::Regex;

pub const REGEX_VALUE: &str = "_{}";
pub const CHUNK_SIZE: usize = 1024 * 1024; // 1024^2 = 1MB

pub fn extract(url: &str) -> Result<(String, u32)> {
    let re = Regex::new(r"_\d{2,}")?;
    let cap = match re.captures_iter(url).last() {
        Some(c) => c,
        None => bail!("Unable to parse `{}`", url),
    };
    let res = &cap[0];

    let url = url.replace(res, REGEX_VALUE);
    let last: u32 = res.replace("_", "").parse()?;

    Ok((url, last))
}

pub fn extract_name(url: &str) -> Result<String> {
    let re = Regex::new(r"\w+_")?;
    let cap = match re.captures_iter(&url).last() {
        Some(c) => c,
        None => bail!("Unable to parse `{}`", url),
    };
    let res = &cap[0];
    let res: Vec<&str> = res.split("_").collect();

    let name = to_title_case(res[0]);

    Ok(name)
}

pub fn fix_num_episode(num: u32) -> String {
    format!("_{:02}", num)
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
