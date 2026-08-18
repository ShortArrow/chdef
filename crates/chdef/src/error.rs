use thiserror::Error;

/// Errors raised while loading or parsing CH / BF definition files.
#[derive(Debug, Error)]
pub enum ChdefError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("CSV parse error at row {row}: {message}")]
    CsvParse { row: usize, message: String },
}

pub type Result<T> = std::result::Result<T, ChdefError>;
