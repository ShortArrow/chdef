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

    /// The bytes handed to a `parse_*_csv_bytes` entry point are not UTF-8.
    /// `valid_up_to` is the offset in those bytes at which decoding stopped,
    /// counting any BOM that was stripped first. Decoding another encoding
    /// is the caller's.
    #[error("input is not valid UTF-8 (valid up to byte {valid_up_to})")]
    Encoding { valid_up_to: usize },
}

pub type Result<T> = std::result::Result<T, ChdefError>;
