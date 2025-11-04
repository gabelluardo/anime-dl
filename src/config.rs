use std::io::Seek;
use std::path::PathBuf;
use std::{fs, io::Read, io::Write};

use anyhow::{Context, Result, anyhow};
use toml_edit::{Document, DocumentMut};

#[cfg(all(not(test), not(windows)))]
fn default_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    path.push(".config/anime-dl/config.toml");
    path
}

#[cfg(all(not(test), windows))]
fn default_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("HOMEPATH").unwrap_or_default());
    path.push(r"AppData\Roaming\anime-dl\config.toml");
    path
}

pub fn load_config(table: &str, key: &str) -> Result<String> {
    let path = default_path();
    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .context("Unable to load configuration")?;

    let mut toml = String::new();
    file.read_to_string(&mut toml)?;

    let doc = toml.parse::<Document<String>>()?;
    let token = doc[table][key]
        .as_str()
        .ok_or_else(|| anyhow!("Unable to load configuration"))?
        .to_string();

    Ok(token)
}

pub fn save_config(table: &str, key: &str, value: &str) -> Result<()> {
    let path = default_path();
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap())?;
    }

    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;

    let mut toml = String::new();
    file.read_to_string(&mut toml)?;
    file.rewind()?;

    let mut doc = toml.parse::<DocumentMut>()?;
    if !toml.contains(table) {
        doc[table] = toml_edit::table();
    }

    doc[table][key] = toml_edit::value(value);
    doc.fmt();

    file.write_all(doc.to_string().as_bytes())
        .context("Unable to write configuration")
}

pub fn clean_config() -> Result<()> {
    let path = default_path();

    fs::remove_file(path).context("Unable to delete configuration")
}

#[cfg(all(test, not(windows)))]
fn default_path() -> PathBuf {
    PathBuf::from("/tmp/adl/test/test.toml")
}

#[cfg(all(test, windows))]
fn default_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("TEMP").unwrap_or_default());
    path.push(r"adl\test\test.toml");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let data = "data test config";
        let res = save_config("test", "test", data);
        assert!(res.is_ok());

        let res = load_config("test", "test");
        assert!(res.is_ok());
        assert_eq!(data, res.unwrap());

        let res = clean_config();
        assert!(res.is_ok());
    }
}
