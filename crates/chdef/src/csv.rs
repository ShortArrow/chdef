use std::io::Read;
use std::path::Path;

use crate::channel::{BitFieldDef, Bound, ChannelDef, DataType};
use crate::columns::{BfColumn, ChColumn, ColumnMap};
use crate::error::{ChdefError, Result};
use crate::issue::{Issue, IssueCode, Parsed};

/// Load a CH CSV from `path` (see [`parse_ch_csv`]). The only part of the
/// crate that reads the filesystem; a consumer holding bytes calls
/// [`parse_ch_csv_bytes`] instead.
pub fn load_ch_csv(path: impl AsRef<Path>) -> Result<Parsed<Vec<ChannelDef>>> {
    parse_ch_csv(&read_to_string(path.as_ref())?)
}

/// Parse CH CSV bytes: strip any leading BOMs, decode as UTF-8, then
/// [`parse_ch_csv`]. Bytes in another encoding are the caller's to decode.
pub fn parse_ch_csv_bytes(bytes: &[u8]) -> Result<Parsed<Vec<ChannelDef>>> {
    parse_ch_csv(decode_utf8(bytes)?)
}

/// Parse CH CSV text into Rows: every readable row, duplicates included,
/// plus the [`Issue`]s found on the way ([`build_layout`] drops the
/// duplicates). Columns are identified by header name in English or Japanese
/// (`docs/spec/format.md` §2); a first row without a `number` column is data, and the
/// first 9 columns are taken in canonical order. Blank rows and rows whose
/// first cell starts with `#` are skipped without an Issue. A column absent
/// from the header is unspecified and raises no Issue; a broken cell in a
/// present column does.
///
/// [`build_layout`]: crate::build_layout
pub fn parse_ch_csv(content: &str) -> Result<Parsed<Vec<ChannelDef>>> {
    Ok(crate::table::ChTable::parse(content)?.channels())
}

/// Interpret the data rows of a CH table: the Rows stage behind
/// [`parse_ch_csv`] and [`crate::ChTable::channels`].
pub(crate) fn interpret_ch(
    map: &ColumnMap<ChColumn>,
    records: &[Vec<String>],
) -> Parsed<Vec<ChannelDef>> {
    let mut issues = assumed_header_issues(map, records, ChColumn::POSITIONAL.len());
    let cell = |record: &[String], column: ChColumn| map.position(column).map(|i| field(record, i));

    let mut channels: Vec<ChannelDef> = Vec::new();
    for (row, record) in records.iter().enumerate() {
        if is_blank(record) || is_comment(record) {
            continue;
        }
        let mut issue = |code: IssueCode, column: ChColumn, message: String| {
            issues.push(Issue {
                code,
                row: Some(row),
                col: map.position(column),
                message,
            });
        };

        let number_cell = cell(record, ChColumn::Number).unwrap_or_default();
        let number = match number_cell.parse::<u32>() {
            Ok(n) if n >= 1 => n,
            _ => {
                issue(
                    IssueCode::ChannelNumberInvalid,
                    ChColumn::Number,
                    format!(
                        "`number` is {}, not an integer >= 1; the row was skipped.",
                        shown(&number_cell)
                    ),
                );
                continue;
            }
        };
        if channels.iter().any(|c| c.number == number) {
            issue(
                IssueCode::ChannelDuplicate,
                ChColumn::Number,
                format!(
                    "channel {number} is already defined; the layout keeps the first definition."
                ),
            );
        }

        let (category, suffix_bytes) = match cell(record, ChColumn::Type) {
            None => (DataType::UI16, None),
            Some(s) => match parse_type(&s) {
                Some(parsed) => parsed,
                None => {
                    issue(
                        IssueCode::TypeAssumed,
                        ChColumn::Type,
                        format!(
                            "`type` is {}, not `UI` / `SI` / `BF` with an optional bit width; UI was assumed.",
                            shown(&s)
                        ),
                    );
                    (DataType::UI16, None)
                }
            },
        };

        let fallback = suffix_bytes.unwrap_or(2);
        let byte_count = match cell(record, ChColumn::Bytes) {
            None => fallback,
            Some(s) => match s.parse::<i64>() {
                Ok(n) if (1..=8).contains(&n) => n as usize,
                Ok(n) => {
                    let clamped = n.clamp(1, 8);
                    issue(
                        IssueCode::BytesOutOfRange,
                        ChColumn::Bytes,
                        format!("`bytes` is {n}, outside 1-8; clamped to {clamped}."),
                    );
                    clamped as usize
                }
                Err(_) => {
                    issue(
                        IssueCode::BytesAssumed,
                        ChColumn::Bytes,
                        format!(
                            "`bytes` is {}, not an integer; {fallback} was assumed.",
                            shown(&s)
                        ),
                    );
                    fallback
                }
            },
        };
        if let Some(implied) = suffix_bytes {
            if implied != byte_count {
                issue(
                    IssueCode::TypeWidthMismatch,
                    ChColumn::Type,
                    format!(
                        "the width suffix of `type` implies {implied} bytes but `bytes` is {byte_count}; `bytes` wins."
                    ),
                );
            }
        }

        let lsb = match cell(record, ChColumn::Lsb) {
            None => 1.0,
            Some(s) if s.is_empty() => 1.0,
            Some(s) => match s.parse::<f64>() {
                Ok(0.0) => 1.0,
                Ok(v) if v.is_finite() => v,
                _ => {
                    issue(
                        IssueCode::LsbInvalid,
                        ChColumn::Lsb,
                        format!("`lsb` is {}, not a finite number; 1 was used.", shown(&s)),
                    );
                    1.0
                }
            },
        };

        let offset = match cell(record, ChColumn::Offset) {
            None => 0.0,
            Some(s) if s.is_empty() => 0.0,
            Some(s) => match s.parse::<f64>() {
                Ok(v) if v.is_finite() => v,
                _ => {
                    issue(
                        IssueCode::OffsetInvalid,
                        ChColumn::Offset,
                        format!("`offset` is {}, not a number; 0 was used.", shown(&s)),
                    );
                    0.0
                }
            },
        };

        let default_value = match cell(record, ChColumn::Default) {
            None => None,
            Some(s) if s.is_empty() => None,
            Some(s) => match parse_raw(&s) {
                Some((v, is_hex)) => {
                    let bits = (byte_count * 8).min(32) as u32;
                    if is_hex && bits < 32 && (v >> bits) != 0 {
                        let masked = v & ((1u32 << bits) - 1);
                        issue(
                            IssueCode::RawOutOfRange,
                            ChColumn::Default,
                            format!(
                                "`default` 0x{v:X} exceeds the {bits}-bit width; the low bits (0x{masked:X}) were used."
                            ),
                        );
                        Some(masked)
                    } else {
                        Some(v)
                    }
                }
                None => {
                    issue(
                        IssueCode::DefaultInvalid,
                        ChColumn::Default,
                        format!(
                            "`default` is {}, neither an integer nor a `0x` value; treated as unspecified.",
                            shown(&s)
                        ),
                    );
                    None
                }
            },
        };

        let bits = (byte_count * 8) as u32;
        let min = match cell(record, ChColumn::Min) {
            None => None,
            Some(s) if s.is_empty() => None,
            Some(s) => match parse_bound_cell(&s, bits) {
                BoundCell::Value(b) => Some(b),
                BoundCell::Overflow { value, masked } => {
                    issue(
                        IssueCode::RawOutOfRange,
                        ChColumn::Min,
                        format!(
                            "`min` 0x{value:X} exceeds the {bits}-bit width; the low bits (0x{masked:X}) were used."
                        ),
                    );
                    Some(Bound::Raw(masked))
                }
                BoundCell::Invalid => {
                    issue(
                        IssueCode::MinInvalid,
                        ChColumn::Min,
                        format!(
                            "`min` is {}, neither a number nor a `0x` value; treated as unspecified.",
                            shown(&s)
                        ),
                    );
                    None
                }
            },
        };
        let max = match cell(record, ChColumn::Max) {
            None => None,
            Some(s) if s.is_empty() => None,
            Some(s) => match parse_bound_cell(&s, bits) {
                BoundCell::Value(b) => Some(b),
                BoundCell::Overflow { value, masked } => {
                    issue(
                        IssueCode::RawOutOfRange,
                        ChColumn::Max,
                        format!(
                            "`max` 0x{value:X} exceeds the {bits}-bit width; the low bits (0x{masked:X}) were used."
                        ),
                    );
                    Some(Bound::Raw(masked))
                }
                BoundCell::Invalid => {
                    issue(
                        IssueCode::MaxInvalid,
                        ChColumn::Max,
                        format!(
                            "`max` is {}, neither a number nor a `0x` value; treated as unspecified.",
                            shown(&s)
                        ),
                    );
                    None
                }
            },
        };

        if let Some(f) = cell(record, ChColumn::Format) {
            if f.eq_ignore_ascii_case("hex") && lsb != 1.0 {
                issue(
                    IssueCode::HexWithLsb,
                    ChColumn::Format,
                    format!(
                        "`format` is HEX but `lsb` is {lsb}; the HEX display shows the raw value, not the physical one."
                    ),
                );
            }
        }

        let ch = ChannelDef {
            number,
            name: cell(record, ChColumn::Name).unwrap_or_default(),
            byte_count,
            data_type: DataType::resolve(category, byte_count),
            lsb,
            offset,
            unit: cell(record, ChColumn::Unit).unwrap_or_default(),
            default_value,
            min,
            max,
        };
        if let (Some(lo), Some(hi)) = (ch.min_value(), ch.max_value()) {
            if lo > hi {
                issue(
                    IssueCode::MinMaxSwapped,
                    ChColumn::Min,
                    format!(
                        "`min` resolves to {lo} but `max` to {hi}; both were kept, and the range matches nothing."
                    ),
                );
            }
        }
        channels.push(ch);
    }

    Parsed {
        value: channels,
        issues,
    }
}

/// Load a BF CSV from `path` (see [`parse_bf_csv`]). The only part of the
/// crate that reads the filesystem; a consumer holding bytes calls
/// [`parse_bf_csv_bytes`] instead.
pub fn load_bf_csv(path: impl AsRef<Path>) -> Result<Parsed<Vec<BitFieldDef>>> {
    parse_bf_csv(&read_to_string(path.as_ref())?)
}

/// Parse BF CSV bytes: strip any leading BOMs, decode as UTF-8, then
/// [`parse_bf_csv`]. Bytes in another encoding are the caller's to decode.
pub fn parse_bf_csv_bytes(bytes: &[u8]) -> Result<Parsed<Vec<BitFieldDef>>> {
    parse_bf_csv(decode_utf8(bytes)?)
}

/// Parse BF CSV text into Rows: every readable row, duplicates included,
/// plus the [`Issue`]s found on the way ([`build_layout`] drops the
/// duplicates). Columns are identified by header name in English or Japanese
/// (`docs/spec/format.md` §2); a first row without a `number` column is data in canonical
/// order. Blank rows and rows whose first cell starts with `#` are skipped
/// without an Issue. Whether a bit fits its parent channel is not checked
/// here — the parent lives in the CH CSV.
///
/// [`build_layout`]: crate::build_layout
pub fn parse_bf_csv(content: &str) -> Result<Parsed<Vec<BitFieldDef>>> {
    Ok(crate::table::BfTable::parse(content)?.bitfields())
}

/// Interpret the data rows of a BF table: the Rows stage behind
/// [`parse_bf_csv`] and [`crate::BfTable::bitfields`].
pub(crate) fn interpret_bf(
    map: &ColumnMap<BfColumn>,
    records: &[Vec<String>],
) -> Parsed<Vec<BitFieldDef>> {
    let mut issues = assumed_header_issues(map, records, BfColumn::CANONICAL.len());
    let cell = |record: &[String], column: BfColumn| map.position(column).map(|i| field(record, i));

    let mut bitfields: Vec<BitFieldDef> = Vec::new();
    for (row, record) in records.iter().enumerate() {
        if is_blank(record) || is_comment(record) {
            continue;
        }
        let mut issue = |code: IssueCode, column: BfColumn, message: String| {
            issues.push(Issue {
                code,
                row: Some(row),
                col: map.position(column),
                message,
            });
        };

        let number_cell = cell(record, BfColumn::Number).unwrap_or_default();
        let parent_channel = match number_cell.parse::<u32>() {
            Ok(n) => n,
            Err(_) => {
                issue(
                    IssueCode::BfParentInvalid,
                    BfColumn::Number,
                    format!(
                        "`number` is {}, not an integer; the row was skipped.",
                        shown(&number_cell)
                    ),
                );
                continue;
            }
        };
        let bit_cell = cell(record, BfColumn::Bit).unwrap_or_default();
        let bit_number = match bit_cell.parse::<u8>() {
            Ok(n) => n,
            Err(_) => {
                issue(
                    IssueCode::BfBitInvalid,
                    BfColumn::Bit,
                    format!(
                        "`bit` is {}, not an integer 0-255; the row was skipped.",
                        shown(&bit_cell)
                    ),
                );
                continue;
            }
        };
        if bitfields
            .iter()
            .any(|b| b.parent_channel == parent_channel && b.bit_number == bit_number)
        {
            issue(
                IssueCode::BfDuplicate,
                BfColumn::Bit,
                format!(
                    "bit {bit_number} of channel {parent_channel} is already defined; the layout keeps the first definition."
                ),
            );
        }

        let default_value = match cell(record, BfColumn::Default) {
            None => None,
            Some(s) if s.is_empty() => None,
            Some(s) => match s.as_str() {
                "0" => Some(0),
                "1" => Some(1),
                _ => {
                    issue(
                        IssueCode::BfDefaultInvalid,
                        BfColumn::Default,
                        format!(
                            "`default` is {}, not 0 or 1; treated as unspecified.",
                            shown(&s)
                        ),
                    );
                    None
                }
            },
        };

        bitfields.push(BitFieldDef {
            parent_channel,
            bit_number,
            name: cell(record, BfColumn::Name).unwrap_or_default(),
            default_value,
        });
    }

    Parsed {
        value: bitfields,
        issues,
    }
}

/// The `header_assumed` Issue when the positional order was assumed for a
/// file that has records. An empty file reports nothing.
fn assumed_header_issues<C: Copy + PartialEq>(
    map: &ColumnMap<C>,
    records: &[Vec<String>],
    columns: usize,
) -> Vec<Issue> {
    if map.assumed && !records.is_empty() {
        vec![Issue {
            code: IssueCode::HeaderAssumed,
            row: None,
            col: None,
            message: format!(
                "No `number` column was found in the first row; the first {columns} columns were taken in canonical order."
            ),
        }]
    } else {
        Vec::new()
    }
}

/// A row whose cells are all empty; skipped without an Issue.
pub(crate) fn is_blank(record: &[String]) -> bool {
    record.iter().all(|c| c.trim().is_empty())
}

/// A row whose first cell starts with `#`; skipped without an Issue.
pub(crate) fn is_comment(record: &[String]) -> bool {
    record
        .first()
        .map(|c| c.trim().starts_with('#'))
        .unwrap_or(false)
}

/// `type` cell → the category (as a representative [`DataType`], resolved to
/// the real width later) and the byte count its width suffix implies, if it
/// has one. `None` when the cell is not `UI` / `SI` / `BF` with an optional
/// bit-width suffix that is a positive multiple of 8.
fn parse_type(s: &str) -> Option<(DataType, Option<usize>)> {
    let category = match s.get(..2)?.to_ascii_uppercase().as_str() {
        "UI" => DataType::UI16,
        "SI" => DataType::SI16,
        "BF" => DataType::BF,
        _ => return None,
    };
    let suffix = &s[2..];
    if suffix.is_empty() {
        return Some((category, None));
    }
    let bits: usize = suffix.parse().ok().filter(|b| *b > 0 && *b % 8 == 0)?;
    Some((category, Some(bits / 8)))
}

/// A `min` / `max` cell as one of its two notations, or the ways it fails:
/// a `0x` value wider than the channel, or text that is neither notation.
enum BoundCell {
    Value(Bound),
    Overflow { value: u64, masked: u64 },
    Invalid,
}

/// Read a `min` / `max` cell: `0x` / `0X` prefix → raw bit pattern checked
/// against `bits`; anything else → finite physical number.
fn parse_bound_cell(s: &str, bits: u32) -> BoundCell {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        match u64::from_str_radix(hex, 16) {
            Ok(v) if bits < 64 && v >> bits != 0 => BoundCell::Overflow {
                value: v,
                masked: v & ((1u64 << bits) - 1),
            },
            Ok(v) => BoundCell::Value(Bound::Raw(v)),
            Err(_) => BoundCell::Invalid,
        }
    } else {
        match s.parse::<f64>() {
            Ok(v) if v.is_finite() => BoundCell::Value(Bound::Physical(v)),
            _ => BoundCell::Invalid,
        }
    }
}

/// A raw-value cell: `0x` / `0X` prefixed hexadecimal or decimal. The flag
/// says it was hexadecimal — only hex values are width-checked.
fn parse_raw(s: &str) -> Option<(u32, bool)> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok().map(|v| (v, true))
    } else {
        s.parse::<u32>().ok().map(|v| (v, false))
    }
}

/// Render a cell for an Issue message: quoted, or the word `empty`.
fn shown(cell: &str) -> String {
    if cell.is_empty() {
        "empty".to_string()
    } else {
        format!("\"{cell}\"")
    }
}

/// Drop every leading BOM and decode the rest as UTF-8. `valid_up_to` of the
/// error counts from the start of `bytes`, BOMs included, so it points into
/// what the caller passed.
pub(crate) fn decode_utf8(bytes: &[u8]) -> Result<&str> {
    const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

    let mut body = bytes;
    while let Some(rest) = body.strip_prefix(&BOM) {
        body = rest;
    }
    let stripped = bytes.len() - body.len();

    std::str::from_utf8(body).map_err(|e| ChdefError::Encoding {
        valid_up_to: stripped + e.valid_up_to(),
    })
}

fn read_to_string(path: &Path) -> Result<String> {
    let mut content = String::new();
    std::fs::File::open(path)
        .and_then(|mut f| f.read_to_string(&mut content))
        .map_err(|source| ChdefError::Io {
            path: path.display().to_string(),
            source,
        })?;
    Ok(content)
}

fn field(record: &[String], index: usize) -> String {
    record
        .get(index)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::IssueCode;

    fn codes(issues: &[Issue]) -> Vec<IssueCode> {
        issues.iter().map(|i| i.code).collect()
    }

    #[test]
    fn parse_ch_csv_from_string() {
        let csv_data =
            "\u{FEFF}番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位\n\
                         1,4,,全般,フレーム番号,UI32,1,,\n\
                         2,2,,全般,ステータス,UI16,1,,\n\
                         3,,,全般,予備,SI8,1,,\n";

        let parsed = parse_ch_csv(csv_data).unwrap();
        let channels = &parsed.value;

        assert_eq!(channels.len(), 3);
        assert_eq!(
            (
                channels[0].number,
                channels[0].byte_count,
                channels[0].data_type.clone()
            ),
            (1, 4, DataType::UI32)
        );
        assert_eq!(
            (
                channels[1].number,
                channels[1].byte_count,
                channels[1].data_type.clone()
            ),
            (2, 2, DataType::UI16)
        );
        assert_eq!(
            (
                channels[2].number,
                channels[2].byte_count,
                channels[2].data_type.clone()
            ),
            (3, 1, DataType::SI8)
        );
        assert_eq!(codes(&parsed.issues), vec![IssueCode::BytesAssumed]);
        assert_eq!(parsed.issues[0].row, Some(2));
    }

    #[test]
    fn parse_ch_csv_reads_default_column_by_header_name() {
        let csv_data = "番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位,値(デフォルト)\n\
                         1,1,,全般,SYNC,UI8,1,,,0x7E\n\
                         2,2,,全般,ステータス,BF,1,,,\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value[0].default_value, Some(0x7E));
        assert_eq!(parsed.value[1].default_value, None);
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn parse_ch_csv_reports_rows_without_numeric_channel() {
        let csv_data =
            "番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位\n\
                         ,,,,(注記行),,,,\n\
                         1,2,,全般,ステータス,UI16,1,,\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value.len(), 1);
        assert_eq!(parsed.issues[0].code, IssueCode::ChannelNumberInvalid);
        assert_eq!(
            (parsed.issues[0].row, parsed.issues[0].col),
            (Some(0), Some(0))
        );
    }

    #[test]
    fn parse_ch_csv_skips_blank_and_comment_rows_without_issue() {
        let csv_data = "number,bytes,name\n\
                         ,,\n\
                         # note to the editor\n\
                         1,2,Status\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value.len(), 1);
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn issue_rows_count_skipped_rows_so_they_map_onto_the_grid() {
        let csv_data = "number,bytes,name\n\
                         ,,\n\
                         x,2,Bad\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(codes(&parsed.issues), vec![IssueCode::ChannelNumberInvalid]);
        assert_eq!(parsed.issues[0].row, Some(1));
    }

    #[test]
    fn parse_ch_csv_reports_non_positive_channel_numbers() {
        let csv_data = "number,bytes,name\n0,2,Zero\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert!(parsed.value.is_empty());
        assert_eq!(codes(&parsed.issues), vec![IssueCode::ChannelNumberInvalid]);
    }

    #[test]
    fn parse_ch_csv_keeps_duplicate_rows_and_reports_them() {
        let csv_data = "number,bytes,name\n1,2,First\n1,4,Second\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value.len(), 2);
        assert_eq!(codes(&parsed.issues), vec![IssueCode::ChannelDuplicate]);
        assert_eq!(parsed.issues[0].row, Some(1));
    }

    #[test]
    fn parse_ch_csv_reports_assumed_and_out_of_range_bytes() {
        let csv_data = "number,bytes,type,name\n\
                         1,,SI8,A\n\
                         2,x,,B\n\
                         3,12,UI,C\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value[0].byte_count, 1);
        assert_eq!(parsed.value[1].byte_count, 2);
        assert_eq!(parsed.value[2].byte_count, 8);
        assert_eq!(
            codes(&parsed.issues),
            vec![
                IssueCode::BytesAssumed,
                IssueCode::TypeAssumed,
                IssueCode::BytesAssumed,
                IssueCode::BytesOutOfRange,
            ]
        );
    }

    #[test]
    fn parse_ch_csv_reports_type_width_mismatch_and_bytes_wins() {
        let csv_data = "number,bytes,type,name\n1,2,UI32,A\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value[0].byte_count, 2);
        assert_eq!(parsed.value[0].data_type, DataType::UI16);
        assert_eq!(codes(&parsed.issues), vec![IssueCode::TypeWidthMismatch]);
    }

    #[test]
    fn parse_ch_csv_reports_invalid_lsb_offset_and_default() {
        let csv_data = "number,lsb,offset,default,name\n1,abc,def,7.5,A\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        let ch = &parsed.value[0];
        assert_eq!((ch.lsb, ch.offset, ch.default_value), (1.0, 0.0, None));
        assert_eq!(
            codes(&parsed.issues),
            vec![
                IssueCode::LsbInvalid,
                IssueCode::OffsetInvalid,
                IssueCode::DefaultInvalid,
            ]
        );
    }

    #[test]
    fn parse_ch_csv_lsb_empty_or_zero_is_one_without_issue() {
        let csv_data = "number,lsb,name\n1,,A\n2,0,B\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value[0].lsb, 1.0);
        assert_eq!(parsed.value[1].lsb, 1.0);
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn parse_ch_csv_reports_hex_default_exceeding_the_width() {
        let csv_data = "number,bytes,default,name\n1,1,0x1FF,A\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value[0].default_value, Some(0xFF));
        assert_eq!(codes(&parsed.issues), vec![IssueCode::RawOutOfRange]);
    }

    #[test]
    fn parse_ch_csv_reports_hex_format_with_scaling_lsb() {
        let csv_data = "number,lsb,format,name\n1,0.1,HEX,A\n2,1,HEX,B\n3,0.1,DEC,C\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(codes(&parsed.issues), vec![IssueCode::HexWithLsb]);
        assert_eq!(parsed.issues[0].row, Some(0));
    }

    #[test]
    fn parse_ch_csv_reads_min_max_as_physical_or_raw() {
        let csv_data = "number,bytes,lsb,offset,min,max,name
                         1,2,0.5,-5,0x10,100,A
                         2,2,1,0,,,B
";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value[0].min, Some(Bound::Raw(0x10)));
        assert_eq!(parsed.value[0].max, Some(Bound::Physical(100.0)));
        assert_eq!((parsed.value[1].min, parsed.value[1].max), (None, None));
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn parse_ch_csv_reports_min_max_that_are_not_values() {
        let csv_data = "number,min,max,name
1,abc,12x,A
";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!((parsed.value[0].min, parsed.value[0].max), (None, None));
        assert_eq!(
            codes(&parsed.issues),
            vec![IssueCode::MinInvalid, IssueCode::MaxInvalid]
        );
    }

    #[test]
    fn parse_ch_csv_reports_swapped_min_max_and_keeps_both() {
        let csv_data = "number,min,max,name
1,10,5,A
";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value[0].min, Some(Bound::Physical(10.0)));
        assert_eq!(parsed.value[0].max, Some(Bound::Physical(5.0)));
        assert_eq!(codes(&parsed.issues), vec![IssueCode::MinMaxSwapped]);
        assert!(!parsed.value[0].range_contains(7.0));
    }

    #[test]
    fn parse_ch_csv_masks_raw_bounds_beyond_the_width() {
        let csv_data = "number,bytes,min,name
1,1,0x1FF,A
";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value[0].min, Some(Bound::Raw(0xFF)));
        assert_eq!(codes(&parsed.issues), vec![IssueCode::RawOutOfRange]);
    }

    #[test]
    fn empty_input_returns_zero_channels_without_issue() {
        let empty = parse_ch_csv("").unwrap();
        assert!(empty.value.is_empty());
        assert!(empty.issues.is_empty());

        let header_only = parse_ch_csv("number,bytes,name\n").unwrap();
        assert!(header_only.value.is_empty());
        assert!(header_only.issues.is_empty());
    }

    #[test]
    fn parse_bf_csv_from_string() {
        let csv_data = "番号,BIT番号,メッセージ名称,値(デフォルト),備考\n\
                         2,0,予備,0,\n\
                         2,1,有効,1,固定値\n";

        let parsed = parse_bf_csv(csv_data).unwrap();
        let bitfields = &parsed.value;

        assert_eq!(bitfields.len(), 2);
        assert_eq!(
            (
                bitfields[0].parent_channel,
                bitfields[0].bit_number,
                bitfields[0].default_value
            ),
            (2, 0, Some(0))
        );
        assert_eq!(
            (
                bitfields[1].parent_channel,
                bitfields[1].bit_number,
                bitfields[1].default_value
            ),
            (2, 1, Some(1))
        );
        assert_eq!(bitfields[1].name, "有効");
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn parse_bf_csv_reports_invalid_parent_bit_and_default() {
        let csv_data = "number,bit,name,default\n\
                         x,0,A,0\n\
                         2,y,B,1\n\
                         2,1,C,2\n";

        let parsed = parse_bf_csv(csv_data).unwrap();

        assert_eq!(parsed.value.len(), 1);
        assert_eq!(parsed.value[0].default_value, None);
        assert_eq!(
            codes(&parsed.issues),
            vec![
                IssueCode::BfParentInvalid,
                IssueCode::BfBitInvalid,
                IssueCode::BfDefaultInvalid,
            ]
        );
    }

    #[test]
    fn parse_bf_csv_keeps_duplicate_bits_and_reports_them() {
        let csv_data = "number,bit,name\n2,1,First\n2,1,Second\n";

        let parsed = parse_bf_csv(csv_data).unwrap();

        assert_eq!(parsed.value.len(), 2);
        assert_eq!(codes(&parsed.issues), vec![IssueCode::BfDuplicate]);
        assert_eq!(parsed.issues[0].row, Some(1));
    }

    #[test]
    fn parse_ch_csv_accepts_english_header_in_any_order() {
        let csv_data = "name,number,type,bytes,lsb,offset,unit,default\n\
                         Frame counter,1,UI,4,1,0,,\n\
                         Temperature,2,SI16,2,0.1,-40,degC,0x00FF\n";

        let parsed = parse_ch_csv(csv_data).unwrap();
        let channels = &parsed.value;

        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].name, "Frame counter");
        assert_eq!((channels[0].number, channels[0].byte_count), (1, 4));
        assert_eq!(channels[0].data_type, DataType::UI32);
        assert_eq!((channels[1].lsb, channels[1].offset), (0.1, -40.0));
        assert_eq!(channels[1].unit, "degC");
        assert_eq!(channels[1].default_value, Some(0xFF));
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn parse_ch_csv_without_header_takes_the_first_nine_columns_in_order() {
        let csv_data = "1,4,,General,Frame counter,UI32,1,,\n\
                         2,2,,General,Status,UI16,1,,\n";

        let parsed = parse_ch_csv(csv_data).unwrap();

        assert_eq!(parsed.value.len(), 2);
        assert_eq!(parsed.value[0].name, "Frame counter");
        assert_eq!(parsed.value[1].byte_count, 2);
        assert_eq!(codes(&parsed.issues), vec![IssueCode::HeaderAssumed]);
        assert_eq!(parsed.issues[0].row, None);
    }

    #[test]
    fn parse_bf_csv_accepts_english_header() {
        let csv_data = "number,bit,name,default,memo\n\
                         2,0,Reserved,0,\n\
                         2,1,Enabled,1,fixed\n";

        let parsed = parse_bf_csv(csv_data).unwrap();

        assert_eq!(parsed.value.len(), 2);
        assert_eq!(parsed.value[1].name, "Enabled");
        assert_eq!(parsed.value[1].default_value, Some(1));
        assert!(parsed.issues.is_empty());
    }

    #[test]
    fn load_ch_csv_reports_missing_file_with_path() {
        let err = load_ch_csv("does/not/exist.csv").unwrap_err();
        assert!(err.to_string().contains("does/not/exist.csv"));
    }

    #[test]
    fn parse_ch_csv_bytes_ignores_leading_boms() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF, 0xEF, 0xBB, 0xBF]);
        bytes.extend_from_slice(b"number,bytes,name\n1,4,Frame counter\n");

        let parsed = parse_ch_csv_bytes(&bytes).unwrap();

        assert_eq!(parsed.value.len(), 1);
        assert_eq!(parsed.value[0].name, "Frame counter");
    }

    #[test]
    fn parse_ch_csv_bytes_reports_where_utf8_decoding_stopped() {
        let mut bytes = b"number,bytes,name\n1,4,".to_vec();
        let invalid_at = bytes.len();
        bytes.extend_from_slice(&[0x94, 0xD4]); // 番 in CP932
        bytes.push(b'\n');

        let err = parse_ch_csv_bytes(&bytes).unwrap_err();

        assert!(matches!(err, ChdefError::Encoding { valid_up_to } if valid_up_to == invalid_at));
    }

    #[test]
    fn parse_ch_csv_bytes_counts_the_bom_it_stripped() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"number,bytes,name\n1,4,");
        let invalid_at = bytes.len();
        bytes.extend_from_slice(&[0x94, 0xD4]);

        let err = parse_ch_csv_bytes(&bytes).unwrap_err();

        assert!(matches!(err, ChdefError::Encoding { valid_up_to } if valid_up_to == invalid_at));
    }

    #[test]
    fn parse_bf_csv_bytes_from_bytes() {
        let bytes = b"number,bit,name,default\n2,1,Enabled,1\n";

        let parsed = parse_bf_csv_bytes(bytes).unwrap();

        assert_eq!(parsed.value.len(), 1);
        assert_eq!(parsed.value[0].name, "Enabled");
    }

    #[test]
    fn load_ch_csv_takes_anything_that_is_a_path() {
        let path = std::env::temp_dir().join("chdef_load_ch_csv_takes_a_path.csv");
        std::fs::write(&path, "number,bytes,name\n1,4,Frame counter\n").unwrap();

        let parsed = load_ch_csv(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(parsed.value[0].name, "Frame counter");
    }
}
