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
    #[error("blocked search request")]
    Search,
    #[error("session ID is required to access this archive")]
    SessionId,
}

#[derive(thiserror::Error, Debug)]
pub enum ScraperError {
    #[error("no name found")]
    Name,
    #[error("no url found")]
    Url,
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: remove this when we'll have proper integration tests
    #[test]
    fn test_session_id_error_display() {
        // Ensures SessionId variant is constructed and its error message matches
        // what the user sees. Also prevents dead_code warnings on the variant.
        let err = RequestError::SessionId;
        assert_eq!(
            err.to_string(),
            "session ID is required to access this archive"
        );
    }
}
