use std::collections::HashSet;
use std::path::PathBuf;

use regex::Regex;

pub use bars::Bars;
pub use bars::ProgressBar;
pub use range::Range;

use crate::errors::{Error, Result};

#[macro_use]
mod macros;

pub mod bars;
pub mod range;
pub mod tui;

pub const PLACEHOLDER: &str = "_{}";

pub struct RegInfo {
    pub name: String,
    pub raw: String,
    pub num: Option<u32>,
}

fn find_first_match(url: &str, matcher: &str) -> Result<String> {
    let re = Regex::new(matcher).unwrap();
    let cap = match re.captures_iter(&url).last() {
        Some(c) => c,
        None => return Err(Error::Parsing(url.to_string())),
    };
    let res = &cap[0];

    Ok(res.to_string())
}

pub fn extract_info(string: &str) -> Result<RegInfo> {
    let name = extract_name(string)?;

    let (raw, num) = match find_first_match(string, r"_\d{2,}") {
        Ok(m) => (
            string.replace(m.as_str(), PLACEHOLDER),
            m.replace("_", "").parse().ok(),
        ),
        _ => (string.to_string(), None),
    };

    Ok(RegInfo { name, raw, num })
}

pub fn extract_name(string: &str) -> Result<String> {
    let m = find_first_match(string, r"\w+[^/]\w+_")?;
    let res = m.split('_').collect::<Vec<_>>();
    let name = to_title_case(res[0]);

    Ok(name)
}

pub fn extract_aw_cookie(string: &str) -> Result<String> {
    let mut m = find_first_match(string, r"AWCookie[A-Za-z]*=[A-Fa-f0-9]+")?;
    m.push_str("; ");

    Ok(m)
}

pub fn to_title_case(s: &str) -> String {
    let mut res = s.to_string();

    let re = Regex::new(r"([A-Z][a-z]+|\d+)").unwrap();
    re.captures_iter(s)
        .map(|c| (&c[0] as &str).to_string())
        .collect::<HashSet<_>>()
        .iter()
        .for_each(|s| res = res.replace(s, &format!(" {}", s)));

    res.trim().to_string()
}

pub fn get_path(args: &crate::cli::Args, url: &str, pos: usize) -> Result<PathBuf> {
    let mut root = args.dir.last().unwrap().to_owned();

    let path = if args.auto_dir {
        let subfolder = self::extract_name(url)?;
        root.push(subfolder);
        root
    } else {
        match args.dir.get(pos) {
            Some(path) => path.to_owned(),
            None => root,
        }
    };

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_info() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let url_raw = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_{}_SUB_ITA.mp4";
        let res: RegInfo = extract_info(url).unwrap();

        assert_eq!(res.name, "Anime Name");
        assert_eq!(res.num, Some(15));
        assert_eq!(res.raw, url_raw);
    }

    #[test]
    fn test_extract_test() {
        let s = r#"<html><script src="/cdn-cgi/apps/head/WvfaYe5SS22u5exoBw70ThuTjHg.js"></script><body><script>document.cookie="AWCookieVerify=295db002e27e3ac26934485002b41564 ; </script></body></html>"#;
        let res = extract_aw_cookie(s).unwrap();

        assert_eq!(res, "AWCookieVerify=295db002e27e3ac26934485002b41564; ")
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

        let s = "SlimeTaoshite300-nen";
        assert_eq!(to_title_case(s), "Slime Taoshite 300-nen");

        let s = "HigeWoSoruSoshiteJoshikouseiWoHirou";
        assert_eq!(
            to_title_case(s),
            "Hige Wo Soru Soshite Joshikousei Wo Hirou"
        )
    }
}
