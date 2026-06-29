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

    #[error("Image decode error: {0}")]
    Decode(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn error_display_messages() {
        let e = Error::Database("connection failed".into());
        assert_eq!(e.to_string(), "Database error: connection failed");

        let e = Error::Ai("model not found".into());
        assert_eq!(e.to_string(), "AI processing error: model not found");

        let e = Error::Other("something".into());
        assert_eq!(e.to_string(), "something");
    }

    #[test]
    fn io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn result_type_works() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<i32> = Err(Error::Config("bad value".into()));
        assert!(err.is_err());
    }
}
