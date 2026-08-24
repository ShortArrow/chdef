//! Channel / bit-field definitions (CH設定 / BF設定).
//!
//! A CH CSV names each contiguous field of a binary frame in order (offset =
//! cumulative byte count); a BF CSV names the individual bits of channels whose
//! type is `BF`. Columns are found by header name, spelled in English or
//! Japanese. This crate parses both files, computes the frame layout, and
//! converts between raw bytes and physical values (`raw * lsb + offset`).
//! A problem in one row never stops loading; it comes back as an [`Issue`]
//! next to the value.

mod channel;
mod columns;
mod csv;
mod error;
#[cfg(feature = "serde")]
pub mod interchange;
mod issue;
mod table;

pub use channel::{
    build_layout, BitFieldDef, ChannelDef, ChannelLayout, DataType, Decoded, Endian, Value,
    ValueDisplay,
};
pub use columns::{BfColumn, ChColumn, ColumnAliases, HeaderLanguage};
pub use csv::{
    load_bf_csv, load_ch_csv, parse_bf_csv, parse_bf_csv_bytes, parse_ch_csv, parse_ch_csv_bytes,
};
pub use error::{ChdefError, Result};
pub use issue::{Issue, IssueCode, Parsed};
pub use table::{BfTable, ChTable, CsvStyle, Grid, LineEnding, Renumbered};
