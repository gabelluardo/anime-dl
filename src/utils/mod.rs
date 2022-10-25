use std::path::PathBuf;

use nom::{
    bytes::complete::take_until,
    character::complete::{alpha0, alphanumeric1, char},
    combinator::map,
    sequence::{preceded, tuple},
    IResult,
};

pub use bars::Bars;
pub use bars::ProgressBar;
pub use range::Range;

use crate::errors::{Error, Result};

#[macro_use]
mod macros;

pub mod bars;
pub mod range;
pub mod tui;

pub fn parse_name(input: &str) -> Result<String> {
    let url = reqwest::Url::parse(input).map_err(|_| Error::Parsing(input.to_owned()))?;
    let res = url
        .path_segments()
        .and_then(|s| s.last())
        .map(|s| s.split('_').collect::<Vec<_>>()[0])
        .ok_or_else(|| Error::Parsing(input.to_owned()))?;

    let name = to_title_case(res);

    Ok(name)
}

pub fn parse_filename(input: &str) -> Result<String> {
    let filename = reqwest::Url::parse(input)
        .map_err(|_| Error::InvalidUrl)?
        .path_segments()
        .and_then(|segments| segments.last())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Parsing(input.to_owned()))?;

    Ok(filename)
}
pub fn parse_aw_cookie(input: &str) -> Result<String> {
    fn aw_parser(input: &str) -> IResult<&str, String> {
        let key = preceded(take_until("AWCookie"), alpha0);
        let value = preceded(char('='), alphanumeric1);
        let parser = tuple((key, value));

        map(parser, |(k, v)| format!("{k}={v}"))(input)
    }

    let (_, mut cookie) = aw_parser(input).map_err(|_| Error::Parsing(input.to_owned()))?;
    cookie.push_str("; ");

    Ok(cookie)
}

pub fn to_title_case(s: &str) -> String {
    let mut v = String::new();
    let mut pos = None;

    for (i, c) in s.char_indices() {
        if let Some(next) = s.chars().nth(i + 1) {
            if i != 0 && c.is_uppercase() && !next.is_uppercase() && !next.is_ascii_digit() {
                v.push(' ')
            }
        }

        // save position of the first digit
        if c.is_ascii_digit() && pos.is_none() {
            pos = Some(v.len());
        }

        v.push(c);
    }

    if let Some(i) = pos {
        v.insert(i, ' ')
    }

    v.to_string()
}

pub fn get_path(args: &crate::cli::Args, url: &str, pos: usize) -> Result<PathBuf> {
    let mut root = args.dir.last().unwrap().to_owned();

    let path = if args.auto_dir {
        let sub_folder = parse_name(url)?;
        root.push(sub_folder);
        root
    } else {
        match args.dir.get(pos) {
            Some(path) => path.to_owned(),
            None => root,
        }
    };

    Ok(path)
}

pub fn is_web_url(s: &str) -> bool {
    reqwest::Url::parse(s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let not_url = "ciao ciao ciao";

        assert!(is_web_url(url));
        assert!(!is_web_url(not_url));
    }

    #[test]
    fn test_extract_cookie() {
        let s = r#"<html><script src="/cdn-cgi/apps/head/WvfaYe5SS22u5exoBw70ThuTjHg.js"></script><body><script>document.cookie="AWCookieVerify=295db002e27e3ac26934485002b41564 ; </script></body></html>"#;
        let res = parse_aw_cookie(s).unwrap();

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
