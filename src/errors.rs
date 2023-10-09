use reqwest::header::InvalidHeaderValue;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RemoteError {
    #[error("Unable to download {0}")]
    Download(String),
    #[error("Unable get data from source\nFrom: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Unable to find a proxy")]
    Proxy,
    #[error("Unable to get data from watching list")]
    WatchingList,
    #[error("No url found")]
    UrlNotFound,
    #[error("No episode found")]
    EpisodeNotFound,
    #[error("No anime found")]
    AnimeNotFound,
}

#[derive(Error, Debug)]
pub enum UserError {
    #[error("Invalid input")]
    InvalidInput,
    #[error("Invalid range")]
    InvalidRange,
    #[error("Invalid token\nFrom: {0}")]
    InvalidToken(#[from] InvalidHeaderValue),
    #[error("Unable to parse `{0}`")]
    Parsing(String),
    #[error("No match found")]
    Choices,
}

#[derive(Error, Debug)]
pub enum SystemError {
    #[error("`mpv` or `vlc` required for streaming")]
    MediaPlayer,
    #[error("{0} already exists")]
    Overwrite(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("Unable to open file")]
    FsOpen,
    #[error("Unable to remove file")]
    FsRemove,
    #[error("Unable to write file")]
    FsWrite,
    #[error("Unable to load configuration")]
    FsLoad,
}

#[derive(Error, Debug)]
pub struct Quit;

impl std::fmt::Display for Quit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
