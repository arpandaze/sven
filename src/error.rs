use thiserror::Error;

#[derive(Error, Debug)]
pub enum SvenError {
    #[error("GPG error: {0}")]
    GpgError(#[from] gpgme::Error),

    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),

    #[error("No GPG keys with ultimate trust found")]
    NoGpgKeys,

    #[error("GPG key not selected")]
    NoKeySelected,

    #[error("GPG not available: {0}")]
    GpgNotAvailable(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Config error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, SvenError>;

