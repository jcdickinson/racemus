use std::error::Error;

#[derive(Debug)]
pub enum ApiError {
    HttpStatus(u16),
}

impl Error for ApiError {}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::HttpStatus(s) => write!(f, "HttpStatus({})", s),
        }
    }
}
