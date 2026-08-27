//! A WebAssembly binding over [`chdef`], for JavaScript and TypeScript.
//!
//! It binds the crate rather than the C ABI: `wasm-bindgen` carries
//! strings, vectors and structs itself and generates the TypeScript
//! declarations, so going through the two-call buffer dance of the C ABI
//! would cost work and buy nothing.
//!
//! The types here are its own. `chdef::Decoded` borrows the layout and the
//! frame it was read from, and a borrowed type cannot leave the module, so
//! a reading is copied into a plain value on the way out — the same shape
//! the .NET binding takes, and the same reason.
//!
//! Two rules shape what leaves.
//!
//! **Only state is a handle.** `Definitions`, `Table` and
//! `ColumnVocabulary` own Rust memory, so they are JavaScript classes with
//! a `free()`. Everything else is a record — a snapshot that owns nothing —
//! and leaves as a plain object, which a caller may keep, spread, clone or
//! post to a worker. A handle into linear memory allows none of those.
//!
//! **A raw bit pattern is a `bigint`.** A JavaScript number holds 53 bits
//! of integer exactly and a channel may be eight bytes wide. A physical
//! value stays a number: it is an `f64` either side.

use chdef::{build_layout, ChTable, ChannelLayout, ColumnVocabulary as Vocabulary, IssueCode};
use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

/// Carry a list of records out as plain objects. Fallible in the type
/// system and not in practice: these are numbers, strings and lists.
fn out<T: Tsify + Serialize>(values: Vec<T>) -> Result<Vec<Ts<T>>, JsError> {
    values.iter().map(|value| Ok(value.into_ts()?)).collect()
}

/// Carry one record out, or nothing.
fn one<T: Tsify + Serialize>(value: Option<T>) -> Result<Option<Ts<T>>, JsError> {
    value.map(|value| Ok(value.into_ts()?)).transpose()
}

/// One channel of the frame.
#[derive(Tsify, Serialize, Clone)]
#[tsify(large_number_types_as_bigints)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub number: u32,
    /// Byte offset of this channel from the start of the frame.
    pub at: u32,
    pub bytes: u32,
    pub name: String,
    /// `UI`, `SI` or `BF`. Further interpretations may appear.
    #[serde(rename = "type")]
    pub data_type: String,
    pub lsb: f64,
    pub offset: f64,
    pub unit: String,
    /// The raw bit pattern this channel takes when no value is given;
    /// `undefined` when the channel states none.
    pub default: Option<u64>,
    pub section: String,
    pub memo: String,
    pub var: String,
    /// `DEC` or `HEX` — which reading is shown, not the base it is
    /// printed in.
    pub format: String,
    pub favorite: bool,
    /// Who decides this channel's value: `plain`, `const`, `counter` or
    /// `derived`.
    pub kind: String,
    /// How a `derived` channel is computed; `""` for every other kind.
    pub derived: String,
    /// The named bits of this channel, in definition order.
    pub bits: Vec<Bit>,
}

/// One named bit of a channel.
#[derive(Tsify, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Bit {
    pub channel: u32,
    pub number: u32,
    pub name: String,
    pub memo: String,
    /// `0` or `1`, or `undefined` when the BF row names none and the bit
    /// keeps the parent channel's.
    pub default: Option<u8>,
}

/// One channel's readings from a decoded frame.
#[derive(Tsify, Serialize, Clone)]
#[tsify(large_number_types_as_bigints)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub channel: u32,
    /// The bit pattern on the wire, read as an integer of the channel's
    /// width. A `bigint`, because a channel may be 64 bits wide.
    pub raw: u64,
    /// `raw × lsb + offset`.
    pub value: f64,
    pub bits: Vec<BitReading>,
}

/// One named bit of a decoded frame, and whether it is set.
#[derive(Tsify, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BitReading {
    pub channel: u32,
    pub number: u32,
    pub name: String,
    pub value: bool,
}

/// A per-row problem found while reading.
///
/// Every field needed to write a sentence is here; `message` is English
/// prose whose wording is not part of the contract.
#[derive(Tsify, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// The stable identifier. `issueCodes()` lists every one.
    pub code: String,
    pub row: Option<u32>,
    pub col: Option<u32>,
    pub channel: Option<u32>,
    pub bit: Option<u8>,
    /// The value chdef could not use, as the file spells it.
    pub found: Option<String>,
    /// The value it used instead, or the bound the finding was measured
    /// against.
    pub used: Option<String>,
    pub message: String,
}

impl Issue {
    fn of(issue: &chdef::Issue) -> Issue {
        Issue {
            code: issue.code.as_str().to_string(),
            row: issue.row.map(|v| v as u32),
            col: issue.col.map(|v| v as u32),
            channel: issue.channel,
            bit: issue.bit,
            found: issue.found.clone(),
            used: issue.used.clone(),
            message: issue.message.clone(),
        }
    }
}

/// Every Issue code this build can report.
///
/// The codes are strings so the vocabulary can grow; this is what lets a
/// table keyed by code be proved complete rather than found short.
#[wasm_bindgen(js_name = "issueCodes")]
pub fn issue_codes() -> Vec<String> {
    IssueCode::all()
        .iter()
        .map(|code| code.as_str().to_string())
        .collect()
}

/// The spellings one caller accepts for the columns of a CH / BF CSV.
///
/// A vocabulary is data, not a language chdef knows: `japanese()` is one
/// value among any number `create()` can build, and has no standing they
/// lack.
#[wasm_bindgen(js_name = "ColumnVocabulary")]
pub struct JsVocabulary {
    inner: Vocabulary,
}

#[wasm_bindgen(js_class = "ColumnVocabulary")]
impl JsVocabulary {
    /// The empty vocabulary: the canonical column names and their
    /// variants alone.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsVocabulary {
        JsVocabulary {
            inner: Vocabulary::new(),
        }
    }

    /// The spellings of the definition files this format was extracted
    /// from.
    pub fn japanese() -> JsVocabulary {
        JsVocabulary {
            inner: Vocabulary::japanese(),
        }
    }

    /// Teach a spelling for a CH column, named by its canonical name. The
    /// **first** spelling taught for a column is the one written.
    /// Returns `false` for a name no column answers to.
    pub fn ch(&mut self, spelling: &str, column: &str) -> bool {
        match chdef::ChColumn::from_header(column) {
            Some(ch) => {
                self.inner = std::mem::take(&mut self.inner).ch(spelling, ch);
                true
            }
            None => false,
        }
    }

    /// Teach a spelling for a BF column. See [`ch`](JsVocabulary::ch).
    pub fn bf(&mut self, spelling: &str, column: &str) -> bool {
        match chdef::BfColumn::from_header(column) {
            Some(bf) => {
                self.inner = std::mem::take(&mut self.inner).bf(spelling, bf);
                true
            }
            None => false,
        }
    }

    /// The canonical names of the CH columns, in canonical order — the
    /// names `ch` is taught against.
    #[wasm_bindgen(js_name = "chColumns")]
    pub fn ch_columns() -> Vec<String> {
        chdef::ChColumn::canonical()
            .iter()
            .map(|c| c.name().to_string())
            .collect()
    }

    /// The canonical names of the BF columns, in canonical order.
    #[wasm_bindgen(js_name = "bfColumns")]
    pub fn bf_columns() -> Vec<String> {
        chdef::BfColumn::canonical()
            .iter()
            .map(|c| c.name().to_string())
            .collect()
    }
}

impl Default for JsVocabulary {
    fn default() -> JsVocabulary {
        JsVocabulary::new()
    }
}

/// A CH / BF definition set read into a frame layout.
#[wasm_bindgen]
pub struct Definitions {
    layout: ChannelLayout,
    issues: Vec<Issue>,
    channels: Vec<Channel>,
}

#[wasm_bindgen]
impl Definitions {
    /// Read a CH CSV and an optional BF CSV.
    ///
    /// A problem in one row never stops the load: it comes back in
    /// `issues`, pointing at the row and column it is about. Throws only
    /// when the *file* cannot be read — bad encoding, an unterminated
    /// quote.
    pub fn parse(
        ch_csv: &str,
        bf_csv: Option<String>,
        vocabulary: Option<JsVocabulary>,
    ) -> Result<Definitions, JsError> {
        let empty = Vocabulary::new();
        let words = vocabulary.as_ref().map(|v| &v.inner).unwrap_or(&empty);

        let channels = chdef::parse_ch_csv_with(ch_csv, words).map_err(to_error)?;
        let bitfields =
            chdef::parse_bf_csv_with(bf_csv.as_deref().unwrap_or(""), words).map_err(to_error)?;

        let built = build_layout(channels.value, bitfields.value);
        let mut issues: Vec<Issue> = channels.issues.iter().map(Issue::of).collect();
        issues.extend(bitfields.issues.iter().map(Issue::of));
        issues.extend(built.issues.iter().map(Issue::of));

        let layout = built.value;
        let channels = read_channels(&layout);
        Ok(Definitions {
            layout,
            issues,
            channels,
        })
    }

    /// The problems found while reading these definitions.
    #[wasm_bindgen(getter)]
    pub fn issues(&self) -> Result<Vec<Ts<Issue>>, JsError> {
        out(self.issues.clone())
    }

    /// The channels, in the order that fixes their positions.
    #[wasm_bindgen(getter)]
    pub fn channels(&self) -> Result<Vec<Ts<Channel>>, JsError> {
        out(self.channels.clone())
    }

    /// The data length of the frame in bytes.
    #[wasm_bindgen(getter, js_name = "totalBytes")]
    pub fn total_bytes(&self) -> u32 {
        self.layout.total_bytes() as u32
    }

    /// Read a frame. A channel that overruns a short frame is omitted, and
    /// so is everything after it — never zero-filled.
    pub fn decode(&self, frame: &[u8]) -> Result<Vec<Ts<Reading>>, JsError> {
        out(self
            .layout
            .decode(frame)
            .iter()
            .map(|reading| Reading {
                channel: reading.channel.number,
                raw: reading.raw,
                value: reading.value,
                bits: reading
                    .bits()
                    .map(|(bit, set)| BitReading {
                        channel: bit.parent_channel,
                        number: bit.bit_number as u32,
                        name: bit.name.clone(),
                        value: set,
                    })
                    .collect(),
            })
            .collect())
    }

    /// Build a frame from `values`. Channels not named take their
    /// default; a derived channel is **not** filled here — sealing is a
    /// call of its own.
    pub fn encode(&self, values: Vec<Ts<Value>>) -> Result<Ts<Encoded>, JsError> {
        let encoded = self.layout.encode(&read_values(values)?);
        Ok(Encoded {
            frame: encoded.value,
            issues: encoded.issues.iter().map(Issue::of).collect(),
        }
        .into_ts()?)
    }

    /// Fill every derived channel of `frame` — the CRCs — and return it.
    ///
    /// A frame is sealed once, after every other value is in place,
    /// because a recipe reads the bytes as they will be sent.
    pub fn seal(&self, frame: &[u8]) -> Result<Ts<Encoded>, JsError> {
        let mut sealed = frame.to_vec();
        let issues = self.layout.seal(&mut sealed);
        Ok(Encoded {
            frame: sealed,
            issues: issues.iter().map(Issue::of).collect(),
        }
        .into_ts()?)
    }

    /// Which derived channels of `frame` disagree with their recipe — the
    /// check a receiver makes. Nothing is changed.
    #[wasm_bindgen(js_name = "derivedMismatches")]
    pub fn derived_mismatches(&self, frame: &[u8]) -> Result<Vec<Ts<Issue>>, JsError> {
        out(self
            .layout
            .derived_mismatches(frame)
            .iter()
            .map(Issue::of)
            .collect())
    }

    /// The bytes a derived channel's recipe covers, in the order it covers
    /// them — the storey below sealing. `undefined` when the channel is
    /// not derived, its recipe was unreadable, or the frame is too short.
    #[wasm_bindgen(js_name = "coveredBytes")]
    pub fn covered_bytes(&self, channel: u32, frame: &[u8]) -> Option<Vec<u8>> {
        self.layout.covered_bytes(channel, frame)
    }

    /// Byte order of every multi-byte channel: `"little"` or `"big"`.
    /// Not written in the CSV; the consumer sets it.
    #[wasm_bindgen(setter)]
    pub fn set_endian(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "\"little\" | \"big\"")] endian: &str,
    ) -> Result<(), JsError> {
        self.layout.endian = match endian {
            "little" => chdef::Endian::Little,
            "big" => chdef::Endian::Big,
            other => return Err(JsError::new(&format!("unknown byte order {other:?}"))),
        };
        Ok(())
    }

    /// The maximum byte count of the data part, for `limitsExceeded`.
    #[wasm_bindgen(setter)]
    pub fn set_capacity(&mut self, capacity: u32) {
        self.layout.capacity = Some(capacity as usize);
    }

    /// The maximum number of channels the port accepts — the limit a byte
    /// count cannot express.
    #[wasm_bindgen(setter, js_name = "channelCapacity")]
    pub fn set_channel_capacity(&mut self, channels: u32) {
        self.layout.channel_capacity = Some(channels as usize);
    }

    /// The findings when the layout does not fit the limits stated; empty
    /// when it fits or none was stated.
    #[wasm_bindgen(js_name = "limitsExceeded")]
    pub fn limits_exceeded(&self) -> Result<Vec<Ts<Issue>>, JsError> {
        out(self
            .layout
            .limits_exceeded()
            .iter()
            .map(Issue::of)
            .collect())
    }

    /// Which of `values` fall outside their channel's declared range.
    /// Nothing is changed and nothing is remembered.
    #[wasm_bindgen(js_name = "valuesOutOfRange")]
    pub fn values_out_of_range(&self, values: Vec<Ts<Value>>) -> Result<Vec<Ts<Issue>>, JsError> {
        let given = read_values(values)?;
        out(self
            .layout
            .values_out_of_range(&given)
            .iter()
            .map(Issue::of)
            .collect())
    }

    /// Which readings of `frame` fall outside their channel's declared
    /// range — the same question, asked of a frame that has arrived.
    #[wasm_bindgen(js_name = "readingsOutOfRange")]
    pub fn readings_out_of_range(&self, frame: &[u8]) -> Result<Vec<Ts<Issue>>, JsError> {
        out(self
            .layout
            .readings_out_of_range(&self.layout.decode(frame))
            .iter()
            .map(Issue::of)
            .collect())
    }

    /// Which of the two readings the channel's `format` column selects
    /// (`DEC` the physical value, `HEX` the raw one). It affects no
    /// conversion.
    pub fn displayed(&self, channel: u32, raw: u64) -> Result<Option<Ts<Value>>, JsError> {
        one(self
            .channel(channel)
            .map(|ch| Value::of(channel, ch.displayed_value(raw))))
    }

    /// The default text form of a reading — the physical value with the
    /// channel's unit, or the raw one in hexadecimal padded to the
    /// channel's width. A caller with its own digit counts writes its own.
    pub fn render(&self, channel: u32, raw: u64) -> Option<String> {
        self.channel(channel).map(|ch| ch.render(raw))
    }

    /// Whether the channel width can hold `value` as given — the ask
    /// before sending, since a value it cannot hold is clamped.
    #[wasm_bindgen(js_name = "fitsWidth")]
    pub fn fits_width(&self, channel: u32, value: f64) -> bool {
        self.channel(channel)
            .map(|ch| ch.fits_width(value))
            .unwrap_or(false)
    }

    /// The declared range of a channel as physical numbers, resolved with
    /// its current `lsb` and `offset`. Either side may be `undefined`.
    #[wasm_bindgen(js_name = "rangeOf")]
    pub fn range_of(&self, channel: u32) -> Result<Option<Ts<Range>>, JsError> {
        one(self.channel(channel).map(|ch| Range {
            min: ch.min_value(),
            max: ch.max_value(),
        }))
    }

    /// One physical value to the raw bit pattern of its channel's width:
    /// rounded half away from zero and clamped to the width, a negative
    /// result as its two's-complement pattern. `undefined` for a value
    /// that cannot be converted — `NaN`, infinite — or a channel the
    /// layout does not have. Clamping is silent here; `fitsWidth` is the
    /// ask.
    #[wasm_bindgen(js_name = "toRaw")]
    pub fn to_raw(&self, channel: u32, value: f64) -> Option<u64> {
        self.channel(channel).and_then(|ch| ch.value_to_raw(value))
    }

    /// One raw bit pattern to the physical value of its channel:
    /// sign-extended for `SI`, then scaled by `lsb` and moved by `offset`.
    /// Bits beyond the channel's width are ignored. `undefined` for a
    /// channel the layout does not have.
    #[wasm_bindgen(js_name = "toValue")]
    pub fn to_value(&self, channel: u32, raw: u64) -> Option<f64> {
        self.channel(channel).map(|ch| ch.raw_to_value_u64(raw))
    }
}

impl Definitions {
    fn channel(&self, number: u32) -> Option<&chdef::ChannelDef> {
        self.layout.channels.iter().find(|c| c.number == number)
    }
}

/// The declared range of a channel, resolved to physical numbers.
#[derive(Tsify, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// The derivation recipes this build knows by name.
///
/// The set can grow, and a recipe naming something outside it is still
/// read — its coverage is available through `coveredBytes`.
#[wasm_bindgen]
pub fn recipes() -> Vec<String> {
    chdef::DerivedRecipe::all()
        .iter()
        .map(|name| name.to_string())
        .collect()
}

// ------------------------------------------------------------- a value

/// A value for one channel: a physical value, or a raw bit pattern.
///
/// The two are distinguished by `form` rather than by which field is
/// filled, so neither a missing field nor both at once can be written.
#[derive(Tsify, Serialize, Deserialize, Clone, Copy)]
#[tsify(large_number_types_as_bigints)]
#[serde(tag = "form", rename_all = "camelCase")]
pub enum Value {
    /// Converted by the channel's `lsb` and `offset`.
    Physical { channel: u32, value: f64 },
    /// Written to the wire as given. A `bigint`, because a channel may be
    /// 64 bits wide.
    Raw { channel: u32, bits: u64 },
}

/// Read the text form of a value (`docs/spec/format.md` §3): a leading
/// `0x` or `0X` is a raw bit pattern, anything else a physical value.
///
/// `undefined` for text that denotes no value — what a field the user is
/// still typing into needs. Every other value is an object literal.
#[wasm_bindgen(js_name = "parseValue")]
pub fn parse_value(text: &str, channel: u32) -> Result<Option<Ts<Value>>, JsError> {
    one(chdef::Value::parse(text).map(|read| Value::of(channel, read)))
}

/// Read the values a caller passed. JavaScript is dynamic, so a value
/// that does not have the declared shape is thrown back rather than
/// silently taken as zero.
fn read_values(values: Vec<Ts<Value>>) -> Result<Vec<(u32, chdef::Value)>, JsError> {
    values
        .into_iter()
        .map(|value| Ok(value.to_rust()?.inner()))
        .collect()
}

impl Value {
    fn inner(self) -> (u32, chdef::Value) {
        match self {
            Value::Physical { channel, value } => (channel, chdef::Value::Physical(value)),
            Value::Raw { channel, bits } => (channel, chdef::Value::Raw(bits)),
        }
    }

    fn of(channel: u32, value: chdef::Value) -> Value {
        match value {
            chdef::Value::Physical(value) => Value::Physical { channel, value },
            chdef::Value::Raw(bits) => Value::Raw { channel, bits },
        }
    }
}

/// A frame and what was found while building it.
#[derive(Tsify, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Encoded {
    #[serde(with = "serde_bytes")]
    #[tsify(type = "Uint8Array")]
    pub frame: Vec<u8>,
    pub issues: Vec<Issue>,
}

// ------------------------------------------------------------ the table

/// A definition file as its cells, plus the columns its header names.
///
/// The cells are what a grid editor draws — comment rows, blank rows and
/// unknown columns included — and they write back in the shape they were
/// read.
#[wasm_bindgen]
pub struct Table {
    inner: ChTable,
}

#[wasm_bindgen]
impl Table {
    /// Read a CH CSV as cells. Throws only when the file cannot be read.
    pub fn parse(text: &str, vocabulary: Option<JsVocabulary>) -> Result<Table, JsError> {
        let empty = Vocabulary::new();
        let words = vocabulary.as_ref().map(|v| &v.inner).unwrap_or(&empty);
        Ok(Table {
            inner: ChTable::parse_with(text, words).map_err(to_error)?,
        })
    }

    /// The header cells, or an empty list for a file read without one.
    #[wasm_bindgen(getter)]
    pub fn header(&self) -> Vec<String> {
        self.inner
            .header()
            .map(<[String]>::to_vec)
            .unwrap_or_default()
    }

    /// How many data rows the file has, comment and blank rows included.
    #[wasm_bindgen(getter, js_name = "rowCount")]
    pub fn row_count(&self) -> u32 {
        self.inner.row_count() as u32
    }

    /// How many cells the data row at `row` has.
    #[wasm_bindgen(js_name = "columnCount")]
    pub fn column_count(&self, row: u32) -> u32 {
        self.inner
            .row(row as usize)
            .map(<[String]>::len)
            .unwrap_or(0) as u32
    }

    /// One data row, in order.
    pub fn row(&self, row: u32) -> Vec<String> {
        self.inner
            .row(row as usize)
            .map(<[String]>::to_vec)
            .unwrap_or_default()
    }

    /// One data cell, 0-based with the header excluded — the row
    /// numbering an Issue uses. Empty outside the grid.
    pub fn cell(&self, row: u32, col: u32) -> String {
        self.inner
            .cell(row as usize, col as usize)
            .unwrap_or_default()
            .to_string()
    }

    /// Overwrite one data cell; a row shorter than `col` is padded with
    /// empty cells. A row outside the grid is ignored.
    #[wasm_bindgen(js_name = "setCell")]
    pub fn set_cell(&mut self, row: u32, col: u32, value: &str) {
        self.inner.set_cell(row as usize, col as usize, value);
    }

    /// Insert an empty data row at `at`, clamped to the end.
    #[wasm_bindgen(js_name = "insertRow")]
    pub fn insert_row(&mut self, at: u32) {
        self.inner.insert_row(at as usize, Vec::new());
    }

    /// Append an empty data row after the last.
    #[wasm_bindgen(js_name = "appendRow")]
    pub fn append_row(&mut self) {
        self.inner.append_row(Vec::new());
    }

    /// Remove the data row at `at`; `false` when the grid has no such row.
    #[wasm_bindgen(js_name = "removeRow")]
    pub fn remove_row(&mut self, at: u32) -> bool {
        self.inner.remove_row(at as usize).is_some()
    }

    /// The file as text, in the shape it was read in.
    #[wasm_bindgen(js_name = "toCsv")]
    pub fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    /// The problems in the current cells, re-read every time.
    #[wasm_bindgen(getter)]
    pub fn issues(&self) -> Result<Vec<Ts<Issue>>, JsError> {
        out(self.inner.channels().issues.iter().map(Issue::of).collect())
    }

    /// Which rows hold a `default` outside that row's own `min` / `max`
    /// (`docs/spec/conversion.md` §8).
    ///
    /// Each finding carries the grid row and the `default` column, so an
    /// editor colours the cell rather than showing a message with no place
    /// to point.
    #[wasm_bindgen(js_name = "defaultsOutOfRange")]
    pub fn defaults_out_of_range(&self) -> Result<Vec<Ts<Issue>>, JsError> {
        out(self
            .inner
            .defaults_out_of_range()
            .iter()
            .map(Issue::of)
            .collect())
    }
}

fn read_channels(layout: &ChannelLayout) -> Vec<Channel> {
    layout
        .positions()
        .map(|(at, ch)| Channel {
            number: ch.number,
            at: at as u32,
            bytes: ch.width() as u32,
            name: ch.name.clone(),
            data_type: ch.data_type.as_str().to_string(),
            lsb: ch.lsb,
            offset: ch.offset,
            unit: ch.unit.clone(),
            default: layout
                .channel_default(ch.number)
                .filter(|_| ch.default_value.is_some() || ch.data_type.is_bitfield()),
            section: ch.section.clone(),
            memo: ch.memo.clone(),
            var: ch.var.clone(),
            format: ch.format.as_str().to_string(),
            favorite: ch.favorite,
            kind: ch.kind.as_str().to_string(),
            derived: ch
                .derived
                .as_ref()
                .map(|r| r.to_string())
                .unwrap_or_default(),
            bits: layout
                .bitfields
                .iter()
                .filter(|b| b.parent_channel == ch.number)
                .map(|b| Bit {
                    channel: b.parent_channel,
                    number: b.bit_number as u32,
                    name: b.name.clone(),
                    memo: b.memo.clone(),
                    default: b.default_value,
                })
                .collect(),
        })
        .collect()
}

fn to_error(error: chdef::ChdefError) -> JsError {
    JsError::new(&error.to_string())
}
