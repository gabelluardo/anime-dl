use std::ops::Deref;
use std::path::PathBuf;

use anyhow::{Context, Result};
pub use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use nom::{
    bytes::complete::take_until,
    character::complete::{alpha0, alphanumeric1, char},
    combinator::map,
    sequence::{preceded, tuple},
    IResult,
};

use crate::errors::UserError;

pub fn parse_name(input: &str) -> Result<String> {
    let url = reqwest::Url::parse(input).context(UserError::Parsing(input.to_string()))?;
    url.path_segments()
        .and_then(|s| s.last())
        .map(|s| s.split('_').collect::<Vec<_>>()[0].to_string())
        .context(UserError::Parsing(input.to_string()))
}

pub fn parse_filename(input: &str) -> Result<String> {
    let filename = reqwest::Url::parse(input)?
        .path_segments()
        .and_then(|segments| segments.last())
        .map(|s| s.to_string())
        .context(UserError::Parsing(input.to_string()))?;
    Ok(filename)
}

pub fn parse_aw_cookie<'a>(input: &'a str) -> Result<String> {
    let parser = |input: &'a str| -> IResult<&str, String> {
        let key = preceded(take_until("AWCookie"), alpha0);
        let value = preceded(char('='), alphanumeric1);
        let parser = tuple((key, value));
        map(parser, |(k, v)| format!("{k}={v}"))(input)
    };
    let (_, mut cookie) = parser(input).unwrap_or_default();
    cookie.push_str("; ");
    Ok(cookie)
}

pub fn recase_string(s: &str, separator: char, all_lowercase: bool) -> String {
    let mut v = String::new();
    let mut pos = None;
    for (i, c) in s.char_indices() {
        if let Some(next) = s.chars().nth(i + 1) {
            if i != 0 && c.is_uppercase() && !next.is_uppercase() && !next.is_ascii_digit() {
                v.push(separator)
            }
        }
        // save position of the first digit
        if c.is_ascii_digit() && pos.is_none() {
            pos = Some(v.len());
        }
        v.push(c);
    }
    if let Some(i) = pos {
        v.insert(i, separator)
    }
    if all_lowercase {
        v = v.to_lowercase();
    }
    v
}

pub fn get_path(args: &crate::cli::Args, url: &str) -> Result<PathBuf> {
    let mut path = args.dir.clone();
    if args.auto_dir {
        let name = parse_name(url)?;
        let dir = to_snake_case!(name);
        path.push(dir)
    }
    Ok(path)
}

pub fn is_web_url(s: &str) -> bool {
    reqwest::Url::parse(s).is_ok()
}

pub struct Bars(MultiProgress);

impl Deref for Bars {
    type Target = MultiProgress;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Bars {
    pub fn new() -> Self {
        Self(Self::instance_multi_bars())
    }

    pub fn add_bar(&self) -> ProgressBar {
        self.add(Self::instance_bar())
    }

    fn instance_style() -> ProgressStyle {
        let style = ProgressStyle::default_bar().template("{spinner:.green} [{elapsed:.magenta}] [{bar:20.cyan/blue}] {binary_bytes_per_sec} {bytes:.cyan}/{total_bytes:.blue} ({eta:.magenta}) {msg:.green}");
        style.progress_chars("#>-")
    }

    fn instance_multi_bars() -> MultiProgress {
        let multi = MultiProgress::new();
        // NOTE: fix for flickering bar bug on windows (https://github.com/mitsuhiko/indicatif/issues/143)
        multi.set_move_cursor(cfg!(windows));
        multi
    }

    fn instance_bar() -> ProgressBar {
        let pb = ProgressBar::new(0);
        pb.set_style(Self::instance_style());
        pb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_path() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let mut args = crate::cli::Args::default();

        args.auto_dir = true;
        args.dir = PathBuf::from("root");
        assert_eq!(
            get_path(&args, url).unwrap(),
            PathBuf::from("root/anime_name")
        );

        args.auto_dir = true;
        args.dir = PathBuf::from("custom_root");
        assert_eq!(
            get_path(&args, url).unwrap(),
            PathBuf::from("custom_root/anime_name")
        );

        args.auto_dir = false;
        args.dir = PathBuf::from("root");
        assert_eq!(get_path(&args, url).unwrap(), PathBuf::from("root"));

        args.auto_dir = false;
        args.dir = PathBuf::from("custom_root");
        assert_eq!(get_path(&args, url).unwrap(), PathBuf::from("custom_root"))
    }

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

        assert_eq!(res, "AWCookieVerify=295db002e27e3ac26934485002b41564; ");

        let s = r#"<html><script src="/cdn-cgi/apps/head/WvfaYe5SS22u5exoBw70ThuTjHg.js"></script><body><script>document.cookie=" ; </script></body></html>"#;
        let res = parse_aw_cookie(s).unwrap();

        assert_eq!(res, "; ")
    }
}
