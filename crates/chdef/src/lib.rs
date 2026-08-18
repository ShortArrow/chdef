//! Channel / bit-field definitions (CH設定 / BF設定).
//!
//! A CH CSV names each contiguous field of a binary frame in order (offset =
//! cumulative byte count); a BF CSV names the individual bits of channels whose
//! type is `BF`. Columns are found by header name, spelled in English or
//! Japanese. This crate parses both files, computes the frame layout, and
//! converts between raw bytes and physical values (`raw * lsb + offset`).

pub mod channel;
pub mod columns;
pub mod csv;
pub mod error;

pub use channel::{build_layout, BitFieldDef, ChannelDef, ChannelLayout, DataType, Endian};
pub use columns::{BfColumn, ChColumn, ColumnMap, HeaderLanguage};
pub use csv::{load_bf_csv, load_ch_csv, parse_bf_csv, parse_ch_csv};
pub use error::{ChdefError, Result};
