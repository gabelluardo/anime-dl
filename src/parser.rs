use anyhow::{Result, anyhow};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct InfoNum {
    pub value: u32,
    pub alignment: usize,
}

pub fn parse_name(input: &str) -> Result<String> {
    let url = reqwest::Url::parse(input)?;
    url.path_segments()
        .and_then(|mut s| s.next_back())
        .and_then(|s| s.split('_').next())
        .map(|s| s.into())
        .ok_or(anyhow!("Unable to parse {input}"))
}

pub fn parse_filename(input: &str) -> Result<String> {
    reqwest::Url::parse(input)?
        .path_segments()
        .and_then(|mut s| s.next_back())
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
    let chars = input.chars().collect::<Vec<_>>();

    let positions = chars
        .windows(2)
        .enumerate()
        .filter_map(|(i, window)| match window {
            ['_', c] if c.is_ascii_digit() => Some(i),
            [c, '_'] if c.is_ascii_digit() => Some(i),
            _ => None,
        })
        .collect::<Vec<_>>();

    match positions.as_slice() {
        [start_idx, end_idx] => {
            let sub_str = input[*start_idx..*end_idx + 1]
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
    num.map(|info| {
        input.replace(
            &format!("{:0fill$}", info.value, fill = info.alignment),
            "{}",
        )
    })
    .unwrap_or(input.into())
}

pub fn recase_string(s: &str, separator: char, all_lowercase: bool) -> String {
    let mut v = String::new();
    let mut pos = None;
    for (i, c) in s.char_indices() {
        if i != 0
            && c.is_uppercase()
            && let Some(next) = s.chars().nth(i + 1)
            && !next.is_uppercase()
            && !next.is_ascii_digit()
        {
            v.push(separator);
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

#[cfg(test)]
mod tests {
    use super::*;

    pub fn is_web_url(s: &str) -> bool {
        reqwest::Url::parse(s).is_ok()
    }

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
