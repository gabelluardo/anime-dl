use crate::Error;
use regex::Regex;

pub const REGEX_VALUE: &str = "_{}_";

pub fn extract(url: &str) -> Error<(String, u32)> {
    let re = Regex::new(r"_\d+_")?;
    let end = re.captures(url).unwrap();

    let url = re.replace_all(url, REGEX_VALUE).to_string();
    let end: u32 = end[0].replace("_", "").parse()?;

    Ok((url, end))
}
