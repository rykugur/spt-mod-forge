// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String),
    #[error("SPT path error: {0}")]
    SptPath(String),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("token required: {0}")]
    TokenRequired(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

// Failing test (will pass once we have the variants)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_displays_nicely() {
        let e = AppError::TokenRequired("FORGE_API_TOKEN missing. Export it or paste at prompt.".into());
        assert!(e.to_string().contains("FORGE_API_TOKEN"));
    }
}
