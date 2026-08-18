use std::io::Read;

use crate::channel::{BitFieldDef, ChannelDef, DataType};
use crate::columns::{BfColumn, ChColumn, ColumnMap};
use crate::error::{ChdefError, Result};

/// Load a CH CSV from `path` (see [`parse_ch_csv`]).
pub fn load_ch_csv(path: &str) -> Result<Vec<ChannelDef>> {
    parse_ch_csv(&read_to_string(path)?)
}

/// Parse CH CSV text. A leading \u{FEFF} is ignored. Columns are identified
/// by header name in English or Japanese ([`ChColumn`]); a first row without
/// a `number` column is data, and the first 9 columns are taken in canonical
/// order. Rows whose `number` is not an integer are skipped; a missing
/// `bytes` falls back to the width implied by `type`.
pub fn parse_ch_csv(content: &str) -> Result<Vec<ChannelDef>> {
    let (map, records) =
        records_with_columns(content, ColumnMap::ch_from_header, ColumnMap::ch_positional)?;
    let cell = |record: &csv::StringRecord, column: ChColumn| {
        map.position(column)
            .map(|i| field(record, i))
            .unwrap_or_default()
    };

    let mut channels = Vec::new();
    for record in &records {
        let number: u32 = match cell(record, ChColumn::Number).parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let (parsed_type, type_bytes) =
            DataType::parse(&cell(record, ChColumn::Type)).unwrap_or((DataType::UI16, 2));
        let byte_count = cell(record, ChColumn::Bytes)
            .parse::<usize>()
            .unwrap_or(type_bytes);
        let data_type = DataType::resolve(parsed_type, byte_count);

        channels.push(ChannelDef {
            number,
            name: cell(record, ChColumn::Name),
            byte_count,
            data_type,
            lsb: cell(record, ChColumn::Lsb).parse::<f64>().unwrap_or(0.0),
            offset: cell(record, ChColumn::Offset).parse::<f64>().unwrap_or(0.0),
            unit: cell(record, ChColumn::Unit),
            default_value: parse_uint(&cell(record, ChColumn::Default)),
        });
    }

    Ok(channels)
}

/// Read every record of `content`. The first row is the header when
/// `from_header` recognises it; otherwise it is data and `positional`
/// supplies the column map.
fn records_with_columns<C: Copy + PartialEq>(
    content: &str,
    from_header: fn(&[&str]) -> Option<ColumnMap<C>>,
    positional: fn() -> ColumnMap<C>,
) -> Result<(ColumnMap<C>, Vec<csv::StringRecord>)> {
    let content = content.trim_start_matches('\u{FEFF}');
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut records = Vec::new();
    for (index, record) in rdr.records().enumerate() {
        records.push(record.map_err(|e| ChdefError::CsvParse {
            row: index + 1,
            message: e.to_string(),
        })?);
    }

    let header_map = records
        .first()
        .and_then(|first| from_header(&first.iter().collect::<Vec<_>>()));
    Ok(match header_map {
        Some(map) => (map, records.split_off(1)),
        None => (positional(), records),
    })
}

/// Parse a CSV cell holding an unsigned integer in either hex (`0x7E`)
/// or decimal form. Returns None on empty / unparsable input so missing
/// defaults stay `None` rather than silently becoming 0.
fn parse_uint(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}

/// Load a BF CSV from `path` (see [`parse_bf_csv`]).
pub fn load_bf_csv(path: &str) -> Result<Vec<BitFieldDef>> {
    parse_bf_csv(&read_to_string(path)?)
}

/// Parse BF CSV text. A leading \u{FEFF} is ignored. Columns are identified
/// by header name in English or Japanese ([`BfColumn`]); a first row without
/// a `number` column is data in canonical order. Rows whose `number` or `bit`
/// is not an integer are skipped; a missing default value is 0.
pub fn parse_bf_csv(content: &str) -> Result<Vec<BitFieldDef>> {
    let (map, records) =
        records_with_columns(content, ColumnMap::bf_from_header, ColumnMap::bf_positional)?;
    let cell = |record: &csv::StringRecord, column: BfColumn| {
        map.position(column)
            .map(|i| field(record, i))
            .unwrap_or_default()
    };

    let mut bitfields = Vec::new();
    for record in &records {
        let parent_channel: u32 = match cell(record, BfColumn::Number).parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let bit_number: u8 = match cell(record, BfColumn::Bit).parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        bitfields.push(BitFieldDef {
            parent_channel,
            bit_number,
            name: cell(record, BfColumn::Name),
            default_value: cell(record, BfColumn::Default).parse::<u8>().unwrap_or(0),
        });
    }

    Ok(bitfields)
}

fn read_to_string(path: &str) -> Result<String> {
    let mut content = String::new();
    std::fs::File::open(path)
        .and_then(|mut f| f.read_to_string(&mut content))
        .map_err(|source| ChdefError::Io {
            path: path.to_string(),
            source,
        })?;
    Ok(content)
}

fn field(record: &csv::StringRecord, index: usize) -> String {
    record.get(index).unwrap_or("").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ch_csv_from_string() {
        let csv_data =
            "\u{FEFF}番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位\n\
                         1,4,,全般,フレーム番号,UI32,1,,\n\
                         2,2,,全般,ステータス,UI16,1,,\n\
                         3,,,全般,予備,SI8,1,,\n";

        let channels = parse_ch_csv(csv_data).unwrap();

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
    }

    #[test]
    fn parse_ch_csv_reads_default_column_by_header_name() {
        let csv_data = "番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位,値(デフォルト)\n\
                         1,1,,全般,SYNC,UI8,1,,,0x7E\n\
                         2,2,,全般,ステータス,BF,1,,,\n";

        let channels = parse_ch_csv(csv_data).unwrap();

        assert_eq!(channels[0].default_value, Some(0x7E));
        assert_eq!(channels[1].default_value, None);
    }

    #[test]
    fn parse_ch_csv_skips_rows_without_numeric_channel() {
        let csv_data =
            "番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位\n\
                         ,,,,(注記行),,,,\n\
                         1,2,,全般,ステータス,UI16,1,,\n";

        let channels = parse_ch_csv(csv_data).unwrap();

        assert_eq!(channels.len(), 1);
    }

    #[test]
    fn parse_bf_csv_from_string() {
        let csv_data = "番号,BIT番号,メッセージ名称,値(デフォルト),備考\n\
                         2,0,予備,0,\n\
                         2,1,有効,1,固定値\n";

        let bitfields = parse_bf_csv(csv_data).unwrap();

        assert_eq!(bitfields.len(), 2);
        assert_eq!(
            (
                bitfields[0].parent_channel,
                bitfields[0].bit_number,
                bitfields[0].default_value
            ),
            (2, 0, 0)
        );
        assert_eq!(
            (
                bitfields[1].parent_channel,
                bitfields[1].bit_number,
                bitfields[1].default_value
            ),
            (2, 1, 1)
        );
        assert_eq!(bitfields[1].name, "有効");
    }

    #[test]
    fn parse_ch_csv_accepts_english_header_in_any_order() {
        let csv_data = "name,number,type,bytes,lsb,offset,unit,default\n\
                         Frame counter,1,UI,4,1,0,,\n\
                         Temperature,2,SI16,2,0.1,-40,degC,0x00FF\n";

        let channels = parse_ch_csv(csv_data).unwrap();

        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].name, "Frame counter");
        assert_eq!((channels[0].number, channels[0].byte_count), (1, 4));
        assert_eq!(channels[0].data_type, DataType::UI32);
        assert_eq!((channels[1].lsb, channels[1].offset), (0.1, -40.0));
        assert_eq!(channels[1].unit, "degC");
        assert_eq!(channels[1].default_value, Some(0xFF));
    }

    #[test]
    fn parse_ch_csv_without_header_takes_the_first_nine_columns_in_order() {
        let csv_data = "1,4,,General,Frame counter,UI32,1,,\n\
                         2,2,,General,Status,UI16,1,,\n";

        let channels = parse_ch_csv(csv_data).unwrap();

        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].name, "Frame counter");
        assert_eq!(channels[1].byte_count, 2);
    }

    #[test]
    fn parse_bf_csv_accepts_english_header() {
        let csv_data = "number,bit,name,default,memo\n\
                         2,0,Reserved,0,\n\
                         2,1,Enabled,1,fixed\n";

        let bitfields = parse_bf_csv(csv_data).unwrap();

        assert_eq!(bitfields.len(), 2);
        assert_eq!(bitfields[1].name, "Enabled");
        assert_eq!(bitfields[1].default_value, 1);
    }

    #[test]
    fn load_ch_csv_reports_missing_file_with_path() {
        let err = load_ch_csv("does/not/exist.csv").unwrap_err();
        assert!(err.to_string().contains("does/not/exist.csv"));
    }
}
