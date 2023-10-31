use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::errors::UserError;

pub fn parse_name(input: &str) -> Result<String> {
    let url = reqwest::Url::parse(input)?;
    url.path_segments()
        .and_then(|s| s.last())
        .and_then(|s| s.split('_').next())
        .map(|s| s.into())
        .context(UserError::Parsing(input.into()))
}

pub fn parse_filename(input: &str) -> Result<String> {
    reqwest::Url::parse(input)?
        .path_segments()
        .and_then(|segments| segments.last())
        .map(|s| s.into())
        .context(UserError::Parsing(input.into()))
}

pub fn parse_aw_cookie(input: &str) -> Option<String> {
    input.find("AWCookie").and_then(|start| {
        input[start..]
            .find(";")
            .map(|end| input[start..start + end].trim().to_string() + ";")
    })
}

pub fn parse_path(args: &crate::cli::Args, url: &str) -> Result<PathBuf> {
    let mut path = args.dir.clone();
    if args.auto_dir {
        let name = parse_name(url)?;
        let dir = to_snake_case!(name);
        path.push(dir)
    }
    Ok(path)
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

pub fn is_web_url(s: &str) -> bool {
    reqwest::Url::parse(s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let res = parse_name(url).unwrap();
        assert_eq!(res, "AnimeName")
    }

    #[test]
    fn test_parse_filename() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let res = parse_filename(url).unwrap();
        assert_eq!(res, "AnimeName_Ep_15_SUB_ITA.mp4")
    }

    #[test]
    fn test_parse_aw_cookie() {
        let s = r#"<html><script src="/cdn-cgi/apps/head/WvfaYe5SS22u5exoBw70ThuTjHg.js"></script><body><script>document.cookie="AWCookieVerify=295db002e27e3ac26934485002b41564 ; </script></body></html>"#;
        let res = parse_aw_cookie(s).unwrap();
        assert_eq!(res, "AWCookieVerify=295db002e27e3ac26934485002b41564;");

        let s = r#"<html><script src="/cdn-cgi/apps/head/WvfaYe5SS22u5exoBw70ThuTjHg.js"></script><body><script>document.cookie=" ; </script></body></html>"#;
        let res = parse_aw_cookie(s);
        assert_eq!(res, None)
    }

    #[test]
    fn test_recase_string() {
        let str = "AnimeName";
        let res = recase_string(str, ' ', false);
        assert_eq!(res, "Anime Name");

        let str = "AnimeName";
        let res = recase_string(str, ' ', true);
        assert_eq!(res, "anime name");

        let str = "AnimeName";
        let res = recase_string(str, '_', true);
        assert_eq!(res, "anime_name");

        let str = "AnimeName";
        let res = recase_string(str, '_', false);
        assert_eq!(res, "Anime_Name")
    }

    #[test]
    fn test_get_path() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let mut args = crate::cli::Args {
            auto_dir: true,
            dir: PathBuf::from("root"),
            ..Default::default()
        };

        assert_eq!(
            parse_path(&args, url).unwrap(),
            PathBuf::from("root/anime_name")
        );

        args.auto_dir = true;
        args.dir = PathBuf::from("custom_root");
        assert_eq!(
            parse_path(&args, url).unwrap(),
            PathBuf::from("custom_root/anime_name")
        );

        args.auto_dir = false;
        args.dir = PathBuf::from("root");
        assert_eq!(parse_path(&args, url).unwrap(), PathBuf::from("root"));

        args.auto_dir = false;
        args.dir = PathBuf::from("custom_root");
        assert_eq!(
            parse_path(&args, url).unwrap(),
            PathBuf::from("custom_root")
        )
    }

    #[test]
    fn test_is_web_url() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let not_url = "ciao ciao ciao";
        assert!(is_web_url(url));
        assert!(!is_web_url(not_url));
    }
}
