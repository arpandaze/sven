use std::sync::mpsc::SendError;
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
    
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Channel send error: {0}")]
    ChannelSendError(String),
}

impl<T> From<SendError<T>> for SvenError {
    fn from(err: SendError<T>) -> Self {
        SvenError::ChannelSendError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SvenError>;

