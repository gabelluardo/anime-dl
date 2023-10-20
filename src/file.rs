use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;

use crate::errors::SystemError;

pub struct FileDest {
    pub size: u64,
    pub path: PathBuf,
    pub overwrite: bool,
}

type FileProps<'a> = (&'a Path, &'a str, bool);

impl FileDest {
    pub async fn new(props: FileProps<'_>) -> Result<Self> {
        let (root, filename, overwrite) = props;
        if !root.exists() {
            fs::create_dir_all(&root).await?;
        }

        let mut path = root.to_path_buf();
        path.push(filename);

        let mut size = 0;
        if path.exists() && !overwrite {
            size = fs::File::open(&path).await?.metadata().await?.len();
        }

        Ok(Self {
            size,
            path,
            overwrite,
        })
    }

    pub async fn open(&self) -> Result<fs::File> {
        fs::OpenOptions::new()
            .append(!self.overwrite)
            .truncate(self.overwrite)
            .write(self.overwrite)
            .create(true)
            .open(&self.path)
            .await
            .context(SystemError::FsOpen)
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;

    use super::*;

    #[cfg(not(windows))]
    fn root_path() -> PathBuf {
        PathBuf::from("/tmp/adl/test")
    }

    #[cfg(windows)]
    fn root_path() -> PathBuf {
        let path = PathBuf::from(std::env::var("TEMP").unwrap_or_default());
        path.push(r"adl\test\");
        path
    }

    #[tokio::test]
    async fn test_file() {
        let root = root_path();
        let filename = "test.dest";
        let props = (root.as_path(), filename, true);

        let file = FileDest::new(props).await.unwrap();
        assert_eq!(file.size, 0);

        let mut path = root.clone();
        path.push(filename);

        assert_eq!(file.path, path);

        let mut dest = file.open().await.unwrap();
        dest.write(b"0000").await.unwrap();

        let props = (root.as_path(), filename, false);
        let file = FileDest::new(props).await.unwrap();
        assert_eq!(file.size, 4);
    }
}
