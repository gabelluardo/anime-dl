use super::*;

use bunt::{print, println};
use std::io::prelude::*;

pub struct Choice {
    link: String,
    name: String,
}

impl Choice {
    pub fn from(link: String, name: String) -> Self {
        Self { link, name }
    }
}

pub fn get_choice(choices: Vec<Choice>) -> Result<Vec<String>> {
    Ok(match choices.len() {
        0 => bail!("No match found"),
        1 => vec![choices[0].link.to_string()],
        _ => {
            println!("{$cyan+bold}{} results found{/$}\n", choices.len());
            for i in 0..choices.len() {
                println!("[{[magenta]}] {[green]}", i + 1, choices[i].name);
            }

            print!(
                "\n\
                {$red}==> {/$}\
                {$bold}What to watch (eg: 1 2 3 or 1-3) [default=All]{/$}\n\
                {$red}==> {/$}",
            );
            std::io::stdout().flush()?;

            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;

            let re = Regex::new(r"[^\d]").unwrap();
            let mut multi = re
                .replace_all(&line, " ")
                .split_ascii_whitespace()
                .into_iter()
                .map(|v| v.parse().unwrap_or(1) as usize)
                .filter(|i| i.gt(&0) && i.le(&choices.len()))
                .collect::<Vec<_>>();

            if line.contains(&[',', '-', '.'][..]) {
                let re = Regex::new(r"(?:\d+[-|,|\.]+\d*)").unwrap();
                re.captures_iter(&line)
                    .map(|c| c[0].to_string())
                    .for_each(|s| {
                        multi.extend(
                            Range::<usize>::parse_and_fill(&s, &choices.len() + 1)
                                .unwrap()
                                .range()
                                .collect::<Vec<_>>(),
                        )
                    })
            }

            multi.sort();
            multi.dedup();
            let res = multi
                .iter()
                .map(|i| choices[i - 1].link.to_string())
                .collect::<Vec<_>>();

            match res.len() {
                0 => choices.into_iter().map(|c| c.link).collect::<Vec<_>>(),
                _ => res,
            }
        }
    })
}

pub fn get_token(url: &str) -> Result<String> {
    print!(
        "{$cyan+bold}Anilist Oauth{/$}\n\n\
        {$green}Autenticate to: {/$}\n\
        {[magenta+bold]}\n\n\
        {$red}==> {/$}\
        {$bold}Paste token here: {/$}\n\
        {$red}==> {/$}",
        url
    );
    std::io::stdout().flush()?;

    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_string())
}
