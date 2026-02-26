use std::fs::File;
use std::path::{Path, PathBuf};
use std::{fs, io::Read, io::Write};

use anyhow::{Context, Result};
use toml_edit::Document;

const TABLE_NAME: &str = "anilist";

/// Loads a value from the AniList configuration
pub fn load(key: &str) -> Option<String> {
    let toml = load_toml().ok()?;
    let token = toml.get(TABLE_NAME)?.get(key)?.as_str()?.to_string();

    Some(token)
}

/// Saves a value to the AniList configuration
pub fn save(key: &str, value: &str) -> Result<()> {
    let path = config_path();

    let toml = match load_toml() {
        Ok(t) => t,
        Err(_) => {
            if let Some(p) = path.parent() {
                fs::create_dir_all(p)?;
            }

            "".parse::<Document<String>>()?
        }
    };

    let config = {
        let mut doc = toml.into_mut();
        if !doc.contains_table(TABLE_NAME) {
            doc[TABLE_NAME] = toml_edit::table();
        }
        doc[TABLE_NAME][key] = toml_edit::value(value);
        doc.fmt();

        doc.to_string()
    };

    safe_save(&config, &path)?;

    Ok(())
}

/// Loads and parses the TOML configuration file
fn load_toml() -> Result<Document<String>> {
    let path = config_path();

    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let toml = content.parse::<Document<String>>()?;

    Ok(toml)
}

/// Saves content using a temporary file to avoid corruption
fn safe_save(content: &str, path: &Path) -> Result<()> {
    let tmp_path = {
        let mut p = path.to_path_buf();
        p.add_extension("tmp");
        p
    };

    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&tmp_path)?;
        file.write_all(content.as_bytes())?;
    }

    fs::copy(tmp_path, path)?;

    Ok(())
}

/// Deletes the configuration file
pub fn clean() -> Result<()> {
    let path = config_path();

    fs::remove_file(path).context("Unable to delete configuration")
}

#[cfg(not(windows))]
const CONFIG_PATH: &str = ".config/anime-dl/config.toml";

#[cfg(windows)]
const CONFIG_PATH: &str = r"AppData\Roaming\anime-dl\config.toml";

/// Returns the configuration file path for the current OS
fn config_path() -> PathBuf {
    let root = {
        #[cfg(test)]
        {
            #[cfg(windows)]
            let r = std::env::var("TEMP").ok();

            #[cfg(not(windows))]
            let r = Some(String::from("/tmp"));

            r
        }

        #[cfg(not(test))]
        {
            #[cfg(windows)]
            let r = std::env::var("HOMEPATH").ok();

            #[cfg(not(windows))]
            let r = std::env::var("HOME").ok();

            r
        }
    };

    let mut path = PathBuf::new();
    if let Some(r) = root {
        path.push(PathBuf::from(r));
    }
    path.push(CONFIG_PATH);

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DATA: &str = "data test config";

    #[test]
    fn test_save() {
        let res = save("test", TEST_DATA);
        assert!(res.is_ok());

        let res = load("test");
        assert_eq!(TEST_DATA, res.unwrap());
    }

    #[test]
    fn test_clean() {
        let res = save("test", TEST_DATA);
        assert!(res.is_ok());

        let res = clean();
        assert!(res.is_ok());
    }
}
