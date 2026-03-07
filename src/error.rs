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
