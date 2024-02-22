use anyhow::{anyhow, Result};

pub fn parse_name(input: &str) -> Result<String> {
    let url = reqwest::Url::parse(input)?;
    url.path_segments()
        .and_then(|s| s.last())
        .and_then(|s| s.split('_').next())
        .map(|s| s.into())
        .ok_or(anyhow!("Unable to parse {input}"))
}

pub fn parse_filename(input: &str) -> Result<String> {
    reqwest::Url::parse(input)?
        .path_segments()
        .and_then(|segments| segments.last())
        .map(|s| s.into())
        .ok_or(anyhow!("Unable to parse {input}"))
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
    fn test_is_web_url() {
        let url = "https://www.domain.tld/sub/anotherSub/AnimeName/AnimeName_Ep_15_SUB_ITA.mp4";
        let not_url = "ciao ciao ciao";
        assert!(is_web_url(url));
        assert!(!is_web_url(not_url));
    }
}
