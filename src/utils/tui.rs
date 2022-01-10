use std::io::{self, Write};

use bunt::{
    termcolor::{ColorChoice, StandardStream},
    write, writeln,
};

use super::*;

use regex::Regex;

#[derive(Clone)]
pub struct Choice {
    link: String,
    name: String,
}

impl Choice {
    pub fn new(link: String, name: String) -> Self {
        Self { link, name }
    }
}

fn parse_input(line: String, choices: Vec<Choice>) -> Vec<String> {
    let re = Regex::new(r"[^\d]").unwrap();
    let mut multi = re
        .replace_all(&line, " ")
        .split_ascii_whitespace()
        .into_iter()
        .filter_map(|v| v.parse().ok())
        .collect::<Vec<_>>();

    if line.contains(&[',', '-', '.'][..]) {
        let re = Regex::new(r"(\d+[,\-.]+\d*)").unwrap();
        re.captures_iter(&line)
            .map(|c| c[0].to_string())
            .for_each(|s| {
                multi.extend(
                    Range::<usize>::parse_and_fill(&s, choices.len())
                        .unwrap()
                        .expand(),
                )
            })
    }

    multi.sort_unstable();
    multi.dedup();

    match multi.len() {
        0 => choices.into_iter().map(|c| c.link).collect::<Vec<_>>(),
        _ => multi
            .into_iter()
            .filter_map(|i| choices.get(i - 1))
            .map(|c| c.link.to_string())
            .collect::<Vec<_>>(),
    }
}

pub async fn get_choice(choices: Vec<Choice>, query: Option<String>) -> Result<Vec<String>> {
    match choices.len() {
        0 => bail!(Error::Tui),
        1 => Ok(vec![choices[0].link.to_string()]),
        _ => {
            let stream = StandardStream::stdout(ColorChoice::Auto);
            let mut stdout = stream.lock();

            writeln!(
                stdout,
                "{$cyan+bold}{} results found{}{/$}\n",
                choices.len(),
                query.map(|q| format!(" for `{}`", q)).unwrap_or_default()
            )?;
            for (i, c) in choices.iter().enumerate() {
                writeln!(stdout, "[{[magenta]}] {[green]}", i + 1, c.name)?;
            }

            write!(
                stdout,
                "\n\
                {$red}==> {/$}\
                {$bold}What to watch (eg: 1 2 3 or 1-3) [default=All, <q> for exit]{/$}\n\
                {$red}==> {/$}",
            )?;
            stdout.flush()?;

            let mut line = String::new();
            io::stdin().read_line(&mut line)?;

            if line.contains('q') {
                bail!(Error::Quit);
            }

            let urls = parse_input(line, choices);

            if urls.is_empty() {
                bail!(Error::EpisodeNotFound);
            }

            Ok(urls)
        }
    }
}

#[cfg(feature = "anilist")]
pub async fn get_token(url: &str) -> Result<String> {
    let stream = StandardStream::stdout(ColorChoice::Always);
    let mut stdout = stream.lock();

    write!(
        stdout,
        "{$cyan+bold}Anilist Oauth{/$}\n\n\
        {$green}Authenticate to: {/$}\n\
        {[magenta+bold]}\n\n\
        {$red}==> {/$}\
        {$bold}Paste token here: {/$}\n\
        {$red}==> {/$}",
        url
    )?;
    stdout.flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;

    let line = line.trim().to_string();

    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_input() {
        let choices = vec![
            Choice {
                link: "link1".to_string(),
                name: "choice1".to_string(),
            },
            Choice {
                link: "link2".to_string(),
                name: "choice2".to_string(),
            },
            Choice {
                link: "link3".to_string(),
                name: "choice3".to_string(),
            },
            Choice {
                link: "link4".to_string(),
                name: "choice4".to_string(),
            },
            Choice {
                link: "link5".to_string(),
                name: "choice5".to_string(),
            },
            Choice {
                link: "link6".to_string(),
                name: "choice6".to_string(),
            },
        ];

        let line = "1,2,3".to_string();
        assert_eq!(
            parse_input(line, choices.clone()),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string()
            ]
        );

        let line = "1-5".to_string();
        assert_eq!(
            parse_input(line, choices.clone()),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string(),
                "link4".to_string(),
                "link5".to_string(),
            ]
        );

        let line = "1-3, 6".to_string();
        assert_eq!(
            parse_input(line, choices.clone()),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string(),
                "link6".to_string()
            ]
        );

        let line = "1-".to_string();
        assert_eq!(
            parse_input(line, choices.clone()),
            vec![
                "link1".to_string(),
                "link2".to_string(),
                "link3".to_string(),
                "link4".to_string(),
                "link5".to_string(),
                "link6".to_string()
            ]
        );
    }
}
