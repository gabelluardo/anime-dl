use reqwest::header::InvalidHeaderValue;
use rustyline::error::ReadlineError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to download {0}")]
    Download(String),

    #[error("`mpv` or `vlc` required for streaming")]
    MediaPlayer,

    #[error("Unable get data from source\nFrom: {0}")]
    Network(#[from] reqwest::Error),

    #[error("{0} already exists")]
    Overwrite(String),

    #[error("Unable to parse `{0}`")]
    Parsing(String),

    #[error("Unable to find a proxy")]
    Proxy,

    #[error("...")]
    Quit,

    #[error("No match found")]
    Choices,

    #[error("Invalid input")]
    UserInput(ReadlineError),

    // Generic errors
    #[error("{0}")]
    Custom(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    // File system errors
    #[error("Unable to open file")]
    FsOpen,

    #[error("Unable to remove file")]
    FsRemove,

    #[error("Unable to write file")]
    FsWrite,

    // Invalid errors
    #[error("Invalid range")]
    InvalidRange,

    #[error("Invalid token\nFrom: {0}")]
    InvalidToken(#[from] InvalidHeaderValue),

    #[error("Invalid url")]
    InvalidUrl,

    // Not found errors
    #[error("No anime found")]
    AnimeNotFound,

    #[error("No `ANIMEDL_ID` env variable found")]
    EnvNotFound,

    #[error("No episode found")]
    EpisodeNotFound,

    #[error("No url found")]
    UrlNotFound,
}

impl Error {
    pub fn with_msg(msg: &str) -> Self {
        Error::Custom(msg.to_string())
    }
}
