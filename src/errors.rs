use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Unable to download {0}")]
    Download(String),

    #[error("Unable get data from source\nFrom: {0}")]
    Network(#[from] reqwest::Error),

    #[error("{0} already exists")]
    Overwrite(String),

    #[error("Unable to parse `{0}`")]
    Parsing(String),

    #[error("Unable to find a proxy")]
    Proxy,

    #[error("")]
    Quit,

    #[error("No match found")]
    Tui,

    #[error("vlc is required for streaming")]
    Vlc,

    // Generic errors
    #[error("{0}")]
    Custom(String),

    #[error("{0}")]
    IOError(#[from] std::io::Error),

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
    InvalidToken(#[from] reqwest::header::InvalidHeaderValue),

    #[error("Invalid url")]
    InvalidUrl,

    // Not found errors
    #[error("No anime found")]
    AnimeNotFound,

    #[error("No `ANIMEDL_ID` env varibale found")]
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
