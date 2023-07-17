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
