//! The JSON shapes of `docs/spec/interchange.md`, behind the `serde`
//! feature. These types are the frozen wire contract that consumers outside
//! Rust read; they are deliberately separate from the domain types, so a
//! field can be added to `ChannelDef` without moving the JSON, and the JSON
//! can keep its short keys (`n`, `at`) and its own spellings (`DEC` /
//! `HEX`, `min` / `max` in their notation).
//!
//! Serialisation itself is the consumer's: chdef builds the value, the
//! caller hands it to whichever serializer it already uses.

use serde::Serialize;

use crate::channel::{ChannelLayout, Decoded};
use crate::issue::Issue;

/// A whole definition set: the layout, its channels with their positions,
/// the bit fields and the Issues found while loading.
#[derive(Debug, Serialize)]
pub struct Definitions<'l> {
    pub total_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<usize>,
    pub endian: &'static str,
    pub channels: Vec<ChannelJson<'l>>,
    pub bitfields: Vec<BitFieldJson<'l>>,
    pub issues: Vec<IssueJson<'l>>,
}

impl<'l> Definitions<'l> {
    /// Build the definition JSON of `layout`, carrying `issues` as they
    /// were reported.
    pub fn of(layout: &'l ChannelLayout, issues: &'l [Issue]) -> Definitions<'l> {
        Definitions {
            total_bytes: layout.total_bytes(),
            capacity: layout.capacity,
            endian: match layout.endian {
                crate::channel::Endian::Little => "little",
                crate::channel::Endian::Big => "big",
            },
            channels: layout
                .positions()
                .map(|(at, ch)| ChannelJson {
                    n: ch.number,
                    at,
                    bytes: ch.byte_count,
                    ty: ch.data_type.as_str(),
                    section: &ch.section,
                    name: &ch.name,
                    lsb: ch.lsb,
                    offset: ch.offset,
                    unit: &ch.unit,
                    default: ch.default_value,
                    format: match ch.format {
                        crate::channel::ValueDisplay::Physical => "physical",
                        crate::channel::ValueDisplay::Raw => "raw",
                    },
                    min: ch.min.map(|v| v.to_string()).unwrap_or_default(),
                    max: ch.max.map(|v| v.to_string()).unwrap_or_default(),
                    memo: &ch.memo,
                    var: &ch.var,
                    favorite: ch.favorite,
                })
                .collect(),
            bitfields: layout
                .bitfields
                .iter()
                .map(|bf| BitFieldJson {
                    n: bf.parent_channel,
                    bit: bf.bit_number,
                    name: &bf.name,
                    default: bf.default_value,
                    memo: &bf.memo,
                })
                .collect(),
            issues: issues.iter().map(IssueJson::of).collect(),
        }
    }

    /// State the maximum byte count of the data part, when it is not the
    /// layout's own. Without either, the key is absent.
    pub fn with_capacity(mut self, capacity: usize) -> Definitions<'l> {
        self.capacity = Some(capacity);
        self
    }
}

/// One channel with the position the layout gives it.
#[derive(Debug, Serialize)]
pub struct ChannelJson<'l> {
    pub n: u32,
    pub at: usize,
    pub bytes: usize,
    #[serde(rename = "type")]
    pub ty: &'static str,
    pub section: &'l str,
    pub name: &'l str,
    pub lsb: f64,
    pub offset: f64,
    pub unit: &'l str,
    pub default: Option<u64>,
    pub format: &'static str,
    /// The bound in its own notation, or empty when unspecified.
    pub min: String,
    pub max: String,
    pub memo: &'l str,
    pub var: &'l str,
    pub favorite: bool,
}

/// One named bit of a `BF` channel.
#[derive(Debug, Serialize)]
pub struct BitFieldJson<'l> {
    pub n: u32,
    pub bit: u8,
    pub name: &'l str,
    pub default: Option<u8>,
    pub memo: &'l str,
}

/// One diagnostic, with its stable code.
#[derive(Debug, Serialize)]
pub struct IssueJson<'l> {
    pub code: &'static str,
    pub row: Option<usize>,
    pub col: Option<usize>,
    pub channel: Option<u32>,
    pub bit: Option<u8>,
    pub found: Option<&'l str>,
    pub used: Option<&'l str>,
    pub message: &'l str,
}

impl<'l> IssueJson<'l> {
    fn of(issue: &'l Issue) -> IssueJson<'l> {
        IssueJson {
            code: issue.code.as_str(),
            row: issue.row,
            col: issue.col,
            channel: issue.channel,
            bit: issue.bit,
            found: issue.found.as_deref(),
            used: issue.used.as_deref(),
            message: &issue.message,
        }
    }
}

/// The decode result: one entry per channel that fitted the frame.
#[derive(Debug, Serialize)]
pub struct Readings(pub Vec<ReadingJson>);

impl Readings {
    /// Build the value JSON of a decoded frame. A physical value that is
    /// not a number serialises as `null`, which JSON has no other way to
    /// carry.
    pub fn of(decoded: &[Decoded<'_, '_>]) -> Readings {
        Readings(
            decoded
                .iter()
                .map(|d| ReadingJson {
                    n: d.channel.number,
                    raw: d.raw,
                    value: d.value.is_finite().then_some(d.value),
                })
                .collect(),
        )
    }
}

/// One channel's raw and physical reading.
#[derive(Debug, Serialize)]
pub struct ReadingJson {
    pub n: u32,
    pub raw: u64,
    pub value: Option<f64>,
}

/// A cell grid, verbatim: the Table JSON of `docs/spec/interchange.md` §2.
#[derive(Debug, Serialize)]
pub struct TableJson<'t> {
    /// Absent for a file read positionally, which has no header row.
    pub header: Option<&'t [String]>,
    pub rows: &'t [Vec<String>],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{build_layout, ChannelDef, DataType, Endian, Value, ValueDisplay};
    use crate::table::ChTable;

    fn sample() -> crate::channel::ChannelLayout {
        let mut counter = ChannelDef::new(1, 4, DataType::UI);
        counter.name = "Frame counter".into();
        counter.section = "General".into();

        let mut status = ChannelDef::new(2, 2, DataType::BF);
        status.name = "Status".into();
        status.section = "General".into();
        status.default_value = Some(1);
        status.format = ValueDisplay::Raw;

        let mut temp = ChannelDef::new(3, 2, DataType::SI);
        temp.name = "Temperature".into();
        temp.lsb = 0.1;
        temp.unit = "degC".into();
        temp.min = Some(Value::Physical(-40.0));
        temp.max = Some(Value::Raw(0x7FFF));
        temp.favorite = true;

        let mut bit = crate::channel::BitFieldDef::new(2, 0);
        bit.name = "Reserved".into();

        build_layout(vec![counter, status, temp], vec![bit]).value
    }

    #[test]
    fn definitions_json_has_the_documented_shape() {
        let json = serde_json::to_value(Definitions::of(&sample(), &[])).unwrap();

        assert_eq!(json["total_bytes"], 8);
        assert_eq!(json["endian"], "little");
        assert!(json.get("capacity").is_none());

        let first = &json["channels"][0];
        assert_eq!(first["n"], 1);
        assert_eq!(first["at"], 0);
        assert_eq!(first["bytes"], 4);
        assert_eq!(first["type"], "UI");
        assert_eq!(first["section"], "General");
        assert_eq!(first["name"], "Frame counter");
        assert_eq!(first["lsb"], 1.0);
        assert_eq!(first["offset"], 0.0);
        assert_eq!(first["unit"], "");
        assert!(first["default"].is_null());
        assert_eq!(first["format"], "physical");
        assert_eq!(first["min"], "");
        assert_eq!(first["max"], "");
        assert_eq!(first["memo"], "");
        assert_eq!(first["var"], "");
        assert_eq!(first["favorite"], false);

        assert_eq!(json["channels"][1]["at"], 4);
        assert_eq!(json["channels"][1]["type"], "BF");
        assert_eq!(json["channels"][1]["default"], 1);
        assert_eq!(json["channels"][1]["format"], "raw");

        let third = &json["channels"][2];
        assert_eq!(third["at"], 6);
        assert_eq!(third["type"], "SI");
        assert_eq!(third["lsb"], 0.1);
        assert_eq!(third["min"], "-40");
        assert_eq!(third["max"], "0x7FFF");
        assert_eq!(third["favorite"], true);

        assert_eq!(json["bitfields"][0]["n"], 2);
        assert_eq!(json["bitfields"][0]["bit"], 0);
        assert_eq!(json["bitfields"][0]["name"], "Reserved");
        assert!(json["bitfields"][0]["default"].is_null());
        assert_eq!(json["issues"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn definitions_json_carries_a_capacity_only_when_given() {
        let json =
            serde_json::to_value(Definitions::of(&sample(), &[]).with_capacity(246)).unwrap();

        assert_eq!(json["capacity"], 246);
    }

    #[test]
    fn definitions_json_carries_the_issues() {
        let parsed = crate::csv::parse_ch_csv("number,type,name\n1,zzz,A\n").unwrap();
        let layout = build_layout(parsed.value, vec![]).value;

        let json = serde_json::to_value(Definitions::of(&layout, &parsed.issues)).unwrap();

        let issue = &json["issues"][0];
        assert_eq!(issue["code"], "type_assumed");
        assert_eq!(issue["row"], 0);
        assert_eq!(issue["col"], 1);
        assert!(issue["message"].as_str().unwrap().contains("UI"));
    }

    #[test]
    fn readings_json_is_the_decode_result() {
        let mut layout = sample();
        layout.endian = Endian::Big;
        let frame = layout.encode(&[(3, Value::Physical(-12.3))]).value;

        let json = serde_json::to_value(Readings::of(&layout.decode(&frame))).unwrap();

        assert_eq!(json[2]["n"], 3);
        assert_eq!(json[2]["raw"], 65413);
        assert!((json[2]["value"].as_f64().unwrap() - -12.3).abs() < 1e-9);
    }

    #[test]
    fn readings_json_writes_null_for_a_value_that_is_not_a_number() {
        let mut ch = ChannelDef::new(1, 1, DataType::UI);
        ch.lsb = f64::NAN;
        let layout = build_layout(vec![ch], vec![]).value;

        let json = serde_json::to_value(Readings::of(&layout.decode(&[0x01]))).unwrap();

        assert!(json[0]["value"].is_null());
    }

    #[test]
    fn table_json_is_the_verbatim_grid() {
        let table = ChTable::parse_with(
            "番号,バイト数,謎の列\n1,4,keep\n# note\n",
            &crate::ColumnVocabulary::japanese(),
        )
        .unwrap();

        let json = serde_json::to_value(table.to_json()).unwrap();

        assert_eq!(json["header"][0], "番号");
        assert_eq!(json["header"][2], "謎の列");
        assert_eq!(json["rows"][0][2], "keep");
        assert_eq!(json["rows"][1][0], "# note");
    }

    #[test]
    fn table_json_has_no_header_for_a_positional_file() {
        let table = ChTable::parse("1,4,,Sec,Name,UI32,1,,\n").unwrap();

        let json = serde_json::to_value(table.to_json()).unwrap();

        assert!(json["header"].is_null());
        assert_eq!(json["rows"][0][0], "1");
    }
}
