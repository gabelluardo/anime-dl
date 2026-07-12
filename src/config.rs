use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use toml_edit::Document;

/// Loads a value from the configuration
pub fn load(table: &str, key: &str) -> Result<String> {
    let toml = load_toml().context("Unable to read configuration file")?;

    let table = toml
        .get(table)
        .context(format!("Missing configuration table `{table}`"))?;
    let value = table
        .get(key)
        .context(format!("Missing configuration key `{table}.{key}`"))?;
    let string = value.as_str().context(format!(
        "Configuration value `{table}.{key}` is not a string"
    ))?;

    Ok(string.to_string())
}

/// Saves a value in the configuration
pub fn save(table: &str, key: &str, value: &str) -> Result<()> {
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
        if !doc.contains_table(table) {
            doc[table] = toml_edit::table();
        }
        doc[table][key] = toml_edit::value(value);
        doc.fmt();

        doc.to_string()
    };

    safe_save(&config, &path)?;

    Ok(())
}

/// Loads and parses the TOML configuration file
fn load_toml() -> Result<Document<String>> {
    let path = config_path();
    let content = fs::read_to_string(path)?;
    let toml = content.parse::<Document<String>>()?;

    Ok(toml)
}

/// Saves content using a temporary file to avoid corruption
fn safe_save(content: &str, dest: &Path) -> Result<()> {
    let tmp_dest = {
        let mut p = dest.to_path_buf();
        p.add_extension("tmp");
        p
    };

    fs::write(&tmp_dest, content)?;
    fs::copy(&tmp_dest, dest)?;
    fs::remove_file(tmp_dest)?;

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
    use serial_test::file_serial;
    use simple_test_case::test_case;

    use super::*;

    const TEST_DATA: &str = "data test config";
    const TEST_TABLE: &str = "table";

    #[test_case(TEST_DATA; "standard data")]
    #[test_case("value with spaces and !@#$%"; "special chars")]
    #[test_case("アニメダウンロード"; "unicode")]
    #[test_case(""; "empty value")]
    #[test_case("overwritten"; "overwrite")]
    #[test]
    #[file_serial]
    fn test_save_and_load(value: &str) {
        save(TEST_TABLE, "test", value).unwrap();
        // empty value may not be saved as a string by toml_edit
        if !value.is_empty() {
            assert_eq!(load(TEST_TABLE, "test").unwrap(), value);
        }
        clean().unwrap();
    }

    #[test_case(true; "clean existing")]
    #[test_case(false; "clean missing")]
    #[test]
    #[file_serial]
    fn test_clean(exists: bool) {
        if exists {
            save(TEST_TABLE, "test", TEST_DATA).unwrap();
            assert!(clean().is_ok());
        } else {
            clean().ok();
            assert!(clean().is_err());
        }
    }

    #[test_case("nonexistent_key", true; "missing key with file")]
    #[test_case("test", false; "missing file")]
    #[test]
    #[file_serial]
    fn test_load_missing(key: &str, create_file: bool) {
        if create_file {
            save(TEST_TABLE, "test", TEST_DATA).unwrap();
        } else {
            clean().ok();
        }
        assert!(load(TEST_TABLE, key).is_err());
        clean().ok();
    }

    #[test_case(2; "two keys")]
    #[test_case(3; "three keys")]
    #[test]
    #[file_serial]
    fn test_save_multiple_keys(count: usize) {
        for i in 0..count {
            save(TEST_TABLE, &format!("key{i}"), &format!("value{i}")).unwrap();
        }
        for i in 0..count {
            assert_eq!(
                load(TEST_TABLE, &format!("key{i}")).unwrap(),
                format!("value{i}")
            );
        }
        clean().unwrap();
    }

    #[test_case("config.toml"; "ends with config.toml")]
    #[test]
    fn test_config_path(expected_suffix: &str) {
        let path = config_path();
        assert!(path.to_string_lossy().ends_with(expected_suffix));
    }

    #[test_case(TEST_DATA; "save creates dir")]
    #[test]
    #[file_serial]
    fn test_save_creates_config_dir(value: &str) {
        clean().ok();
        save(TEST_TABLE, "test", value).unwrap();
        assert_eq!(load(TEST_TABLE, "test").unwrap(), value);
        clean().unwrap();
    }

    #[test_case(TEST_DATA; "load after clean returns none")]
    #[test]
    #[file_serial]
    fn test_load_after_clean(value: &str) {
        save(TEST_TABLE, "test", value).unwrap();
        clean().unwrap();
        assert!(load(TEST_TABLE, "test").is_err());
    }
}
