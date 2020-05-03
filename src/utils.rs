use anyhow::{bail, Result};
use regex::Regex;

pub const REGEX_VALUE: &str = "_{}_";
pub const CHUNK_SIZE: usize = 1024 * 1024; // 1024^2 = 1MB

pub fn extract(url: &str) -> Result<(String, u32)> {
    let re = Regex::new(r"_\d+_")?;
    let cap = match re.captures_iter(url).last() {
        Some(c) => c,
        None => bail!("Unable to parse `{}`", url),
    };
    let res = &cap[0];

    let url = url.replace(res, REGEX_VALUE);
    let last: u32 = res.replace("_", "").parse()?;

    Ok((url, last))
}

pub fn fix_num_episode(num: u32) -> String {
    format!("_{:02}_", num)
}
