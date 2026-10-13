use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse {path}: {message}")]
    Parse { path: String, message: String },

    #[error("failed to write a record: {0}")]
    Render(String),

    #[error("invalid grain-id `{0}`")]
    InvalidId(String),

    #[error("activity `{0}` not found")]
    ActivityNotFound(String),

    #[error("stamp `{0}` not found")]
    StampNotFound(String),

    #[error("`{0}` already exists")]
    AlreadyExists(String),
}

pub type Result<T> = std::result::Result<T, Error>;
