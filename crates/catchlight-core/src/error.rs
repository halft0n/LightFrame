use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Metadata extraction failed: {0}")]
    Metadata(String),

    #[error("Thumbnail generation failed: {0}")]
    Thumbnail(String),

    #[error("AI processing error: {0}")]
    Ai(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
