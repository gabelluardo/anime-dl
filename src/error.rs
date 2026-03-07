#[derive(thiserror::Error, Debug)]
pub enum TuiError {
    #[error("invalid input")]
    InvalidInput,
}

#[derive(thiserror::Error, Debug)]
pub enum RangeError {
    #[error("invalid range string")]
    Invalid,
}

#[derive(thiserror::Error, Debug)]
pub enum RequestError {
    #[error("unable to get data from watching list")]
    WatchingList,
}
