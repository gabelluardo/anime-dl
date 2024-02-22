use std::path::PathBuf;
use std::{fs, io::Read, io::Write};

use anyhow::{Context, Result};

#[cfg(all(not(test), not(windows)))]
fn default_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    path.push(".config/anime-dl/token");
    path
}

#[cfg(all(not(test), windows))]
fn default_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("HOMEPATH").unwrap_or_default());
    path.push(r"AppData\Roaming\anime-dl\token");
    path
}

pub fn load_config() -> Result<String> {
    let path = default_path();
    let file = fs::OpenOptions::new().read(true).open(path);

    file.map(|mut f| {
        let mut contents = String::new();
        f.read_to_string(&mut contents).ok();
        contents
    })
    .context("Unable to load configuration")
}

pub fn save_config(token: &str) -> Result<()> {
    let path = default_path();
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap())?;
    }
    let mut buf = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    buf.write_all(token.as_bytes())
        .context("Unable to write file")
}

pub fn clean_config() -> Result<()> {
    let path = default_path();

    fs::remove_file(path).context("Unable to remove file")
}

#[cfg(all(test, not(windows)))]
fn default_path() -> PathBuf {
    PathBuf::from("/tmp/adl/test/test.config")
}

#[cfg(all(test, windows))]
fn default_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("TEMP").unwrap_or_default());
    path.push(r"adl\test\test.config");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let data = "data test config";
        let res = save_config(data);
        assert!(res.is_ok());

        let res = load_config();
        assert!(res.is_ok());
        assert_eq!(data, res.unwrap());

        let res = clean_config();
        assert!(res.is_ok());
    }
}
