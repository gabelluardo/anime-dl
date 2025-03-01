use anyhow::{anyhow, Result};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct InfoNum {
    pub value: u32,
    pub alignment: usize,
}

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

pub fn parse_percentage(input: &str) -> Option<u32> {
    let sym = input.find('%')?;

    input[sym - 3..sym]
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

pub fn parse_number(input: &str) -> Option<InfoNum> {
    // find episode number position in input
    let (mut opt_start, mut opt_end) = (None, None);
    for i in 0..input.len() - 1 {
        match (
            input.chars().nth(i).unwrap(),
            input.chars().nth(i + 1).unwrap(),
        ) {
            ('_', next) if next.is_ascii_digit() => opt_start = Some(i),
            (curr, '_') if curr.is_ascii_digit() => opt_end = Some(i),
            _ => continue,
        }
    }

    match (opt_start, opt_end) {
        (Some(start_pos), Some(end_pos)) => {
            let sub_str = input[start_pos..end_pos + 1]
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>();
            sub_str.parse::<u32>().ok().map(|value| InfoNum {
                value,
                alignment: sub_str.len(),
            })
        }
        _ => None,
    }
}

pub fn parse_url(input: &str, num: Option<InfoNum>) -> String {
    match num {
        Some(InfoNum { value, alignment }) => input.replace(&zfill!(value, alignment), "{}"),
        _ => input.into(),
    }
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

pub fn _is_web_url(s: &str) -> bool {
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
        assert!(_is_web_url(url));
        assert!(!_is_web_url(not_url));
    }
}
