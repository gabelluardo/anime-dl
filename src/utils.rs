use aes_soft::Aes128;
use anyhow::{bail, Result};
use block_modes::block_padding::NoPadding;
use block_modes::{BlockMode, Cbc};
use colored::Colorize;
use hex;
use indicatif::*;
use regex::Regex;

use std::io::prelude::*;

pub const REGEX_VALUE: &str = "_{}";

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

    let raw = string.replace(reg_num.as_str(), REGEX_VALUE);
    let num = reg_num.replace("_", "").parse()?;

    Ok(RegInfo { name, raw, num })
}

pub fn find_first_match(url: &str, matcher: &str) -> Result<String> {
    let re = Regex::new(matcher)?;
    let cap = match re.captures_iter(&url).last() {
        Some(c) => c,
        None => bail!("Unable to parse `{}`", url),
    };
    let res = &cap[0];

    Ok(res.to_string())
}

pub fn find_all_match(text: &str, matcher: &str) -> Result<Vec<Vec<u8>>> {
    let re = Regex::new(matcher)?;
    let cap = re
        .captures_iter(&text)
        .map(|c| (&c[0] as &str).to_string())
        .map(|s| {
            s.trim_matches(|c| c == '(' || c == ')' || c == '"')
                .to_string()
        })
        .collect::<Vec<_>>();

    let mut res = vec![];
    for s in cap {
        res.push(hex::decode(s)?);
    }

    Ok(res)
}

pub fn crypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<String> {
    type Aes128Cbc = Cbc<Aes128, NoPadding>;

    let cipher = Aes128Cbc::new_var(&key, &iv)?;
    let out = hex::encode(cipher.decrypt_vec(&data)?);

    Ok(out)
}

pub fn to_title_case(s: &str) -> String {
    let re = Regex::new(r"[A-Z][^A-Z]+").unwrap();
    let mut res = s.to_string();
    re.captures_iter(s)
        .map(|c| (&c[0] as &str).to_string())
        .for_each(|c| res = res.replace(&c, &format!(" {}", c)));

    res.trim().to_string()
}

fn instance_style() -> ProgressStyle {
    ProgressStyle::default_bar().template("{spinner:.green} [{elapsed}] [{bar:35.cyan/blue}] {bytes}/{total_bytes} ({eta}) {wide_msg}").progress_chars("#>-")
}

pub fn instance_multi_bars() -> MultiProgress {
    let multi = MultiProgress::new();

    // for flickering bar bug (https://github.com/mitsuhiko/indicatif/issues/143)
    multi.set_move_cursor(cfg!(windows));
    multi
}

pub fn instance_bar() -> ProgressBar {
    let pb = ProgressBar::new(0);
    pb.set_style(instance_style());
    pb
}

pub fn prompt_choices(choices: Vec<(String, String)>) -> Result<Vec<String>> {
    Ok(match choices.len() {
        0 => bail!("No match found"),
        1 => vec![choices[0].0.to_string()],
        _ => {
            println!(
                "{}",
                format!("{} results found\n", choices.len())
                    .bright_cyan()
                    .bold()
            );
            for i in 0..choices.len() {
                println!(
                    "[{}] {}",
                    format!("{}", i + 1).bright_purple(),
                    format!("{}", choices[i].1).bright_green()
                );
            }

            print!(
                "\n{} {}\n{}",
                format!("==>").bright_red().bold(),
                format!("What to watch (eg: 1 2 3 or 1-3) [default=All]").bold(),
                format!("==> ").bright_red().bold()
            );
            std::io::stdout().flush()?;

            let mut multi = vec![];
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;

            let re = Regex::new(r"[^\d]")?;
            multi.extend(
                re.replace_all(&line, " ")
                    .split_ascii_whitespace()
                    .into_iter()
                    .map(|v| v.parse().unwrap_or(1) as usize)
                    .filter(|i| i.gt(&0) && i.le(&choices.len()))
                    .collect::<Vec<_>>(),
            );

            if line.contains('-') {
                let re = Regex::new(r"(?:\d+\-\d+)")?;
                re.captures_iter(&line)
                    .map(|c| c[0].to_string())
                    .for_each(|s| {
                        let range = s
                            .split('-')
                            .into_iter()
                            .map(|v| v.parse().unwrap_or(1) as usize)
                            .filter(|i| i.gt(&0) && i.le(&choices.len()))
                            .collect::<Vec<_>>();
                        let start = *range.first().unwrap();
                        let end = *range.last().unwrap();

                        multi.extend((start + 1..end).collect::<Vec<_>>())
                    });
            }

            multi.sort();
            let res = multi
                .iter()
                .map(|i| choices[i - 1].0.to_string())
                .collect::<Vec<_>>();

            match res.len() {
                0 => choices
                    .into_iter()
                    .map(|c| c.0.to_string())
                    .collect::<Vec<_>>(),
                _ => res,
            }
        }
    })
}

pub fn format_err(s: anyhow::Error) -> colored::ColoredString {
    format!("[ERR] {}", s).red()
}

// DEPRECATED: since 1.0.0-rc.1
#[allow(dead_code)]
pub fn format_wrn(s: &str) -> colored::ColoredString {
    format!("[WRN] {}", s).yellow()
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
    }
}
