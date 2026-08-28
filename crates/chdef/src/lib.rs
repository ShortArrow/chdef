//! Channel and bit-field definitions for binary frames.
//!
//! The crate front page is the repository readme, included here so that
//! every example on it is compiled and run by `cargo test`. A page that
//! cannot rot is the point: this one is what crates.io shows, and it went
//! stale twice before it was checked.
#![doc = include_str!("../README.md")]

mod channel;
mod columns;
mod csv;
mod derived;
mod error;
#[cfg(feature = "serde")]
pub mod interchange;
mod issue;
mod table;

pub use channel::{
    build_layout, BitFieldDef, ChannelDef, ChannelKind, ChannelLayout, DataType, Decoded, Endian,
    Value, ValueDisplay,
};
pub use columns::{BfColumn, ChColumn, ColumnVocabulary};
pub use csv::{
    load_bf_csv, load_bf_csv_with, load_ch_csv, load_ch_csv_with, parse_bf_csv, parse_bf_csv_bytes,
    parse_bf_csv_bytes_with, parse_bf_csv_with, parse_ch_csv, parse_ch_csv_bytes,
    parse_ch_csv_bytes_with, parse_ch_csv_with,
};
pub use derived::{Crc, Derivation, DerivedRecipe};
pub use error::{ChdefError, Result};
pub use issue::{Issue, IssueCode, Parsed};
pub use table::{BfTable, ChTable, CsvStyle, Grid, LineEnding, Renumbered};
