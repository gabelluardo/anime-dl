pub mod bars;
pub mod crypt;
pub mod tui;

use anyhow::{bail, Result};
use rand::prelude::*;
use regex::Regex;

pub const PLACEHOLDER: &str = "_{}";

pub struct RegInfo {
    pub name: String,
    pub raw: String,
    pub num: u32,
}

pub fn extract_info(string: &str) -> Result<RegInfo> {
    let reg_num = find_first_match(string, r"_\d{2,}")?;
    let reg_name = find_first_match(string, r"\w+[^/]\w+_")?;

    let res = reg_name.split("_").collect::<Vec<_>>();
    let name = to_title_case(res[0]);

    let raw = string.replace(reg_num.as_str(), PLACEHOLDER);
    let num = reg_num.replace("_", "").parse()?;

    Ok(RegInfo { name, raw, num })
}

pub fn find_first_match(url: &str, matcher: &str) -> Result<String> {
    let re = Regex::new(matcher).unwrap();
    let cap = match re.captures_iter(&url).last() {
        Some(c) => c,
        None => bail!("Unable to parse `{}`", url),
    };
    let res = &cap[0];

    Ok(res.to_string())
}

pub fn to_title_case(s: &str) -> String {
    let mut res = s.to_string();

    let re = Regex::new(r"[A-Z][a-z]+").unwrap();
    re.captures_iter(s)
        .map(|c| (&c[0] as &str).to_string())
        .for_each(|c| res = res.replace(&c, &format!(" {}", c)));

    let re = Regex::new(r"\d").unwrap();
    re.captures_iter(s)
        .map(|c| (&c[0] as &str).to_string())
        .for_each(|c| res = res.replace(&c, &format!(" {}", c)));

    res.trim().to_string()
}

pub fn rand_range(low: usize, high: usize) -> usize {
    thread_rng().gen_range(low, high)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_info() {
        let url = "http://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let url_raw = "http://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let res: RegInfo = extract_info(url).unwrap();

        assert_eq!(res.name, "Anime Name");
        assert_eq!(res.num, 15);
        assert_eq!(res.raw, url_raw);
    }

    #[test]
    fn test_to_title_case() {
        let s = "StringaInTitleCase-con-delle-linee";
        assert_eq!(to_title_case(s), "Stringa In Title Case-con-delle-linee");

        let s = "StringaCoNMaiuscole";
        assert_eq!(to_title_case(s), "Stringa CoN Maiuscole");

        let s = "HighSchoolDxD";
        assert_eq!(to_title_case(s), "High School DxD");

        let s = "IDInvaded";
        assert_eq!(to_title_case(s), "ID Invaded");

        let s = "SwordArtOnline2";
        assert_eq!(to_title_case(s), "Sword Art Online 2");

        let s = "SAO2";
        assert_eq!(to_title_case(s), "SAO 2");
    }
}