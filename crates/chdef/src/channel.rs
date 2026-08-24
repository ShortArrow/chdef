use crate::issue::{Issue, IssueCode, Parsed};

/// Byte order of multi-byte channels on the wire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Endian {
    #[default]
    Little,
    Big,
}

/// Interpretation of a channel's bytes. Wider widths may become variants
/// (the specification already names 64-bit suffixes), so matches need a
/// catch-all arm.
/// How a consumer renders the channel's value (`format` column). It never
/// affects a conversion: `raw_to_value` returns the physical value for a
/// `Hex` channel too, and the consumer shows the raw value instead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum DisplayFormat {
    #[default]
    Dec,
    Hex,
}

impl DisplayFormat {
    /// `DEC` / `HEX`, case-insensitively; `None` for anything else.
    pub fn parse(s: &str) -> Option<DisplayFormat> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("dec") {
            Some(DisplayFormat::Dec)
        } else if s.eq_ignore_ascii_case("hex") {
            Some(DisplayFormat::Hex)
        } else {
            None
        }
    }
}

/// A number carrying its notation, as the CSV and consumer input spell it:
/// a plain number is a physical value, a `0x` value is a raw bit pattern.
/// Used by the `min` / `max` bounds (resolved with the channel's current
/// `lsb` / `offset` at query time, so a runtime edit of `lsb` moves them —
/// and never applied by a conversion; the caller opts in via
/// [`ChannelDef::range_contains`] / [`ChannelDef::clamp_to_range`]) and by
/// the per-channel values handed to [`ChannelLayout::encode`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Physical(f64),
    Raw(u64),
}

impl std::fmt::Display for Value {
    /// The notation [`Value::parse`] reads back: a physical value as the
    /// shortest decimal that round-trips, a raw one as `0x` and upper-case
    /// hexadecimal.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Physical(v) => write!(f, "{v}"),
            Value::Raw(r) => write!(f, "0x{r:X}"),
        }
    }
}

impl Value {
    /// Read consumer input (a form field, a cell) by the notation rule:
    /// `0x` / `0X` prefix → raw, any other finite number → physical,
    /// anything else → `None`. Input is trimmed first.
    pub fn parse(s: &str) -> Option<Value> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            u64::from_str_radix(hex, 16).ok().map(Value::Raw)
        } else {
            s.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .map(Value::Physical)
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    UI8,
    UI16,
    UI32,
    SI8,
    SI16,
    SI32,
    BF,
}

impl DataType {
    pub fn byte_count(&self) -> usize {
        match self {
            DataType::UI8 | DataType::SI8 => 1,
            DataType::UI16 | DataType::SI16 | DataType::BF => 2,
            DataType::UI32 | DataType::SI32 => 4,
        }
    }

    pub(crate) fn resolve(parsed: Self, byte_count: usize) -> Self {
        match parsed {
            DataType::BF => DataType::BF,
            ref dt if dt.category() == "SI" => match byte_count {
                1 => DataType::SI8,
                4 => DataType::SI32,
                _ => DataType::SI16,
            },
            _ => match byte_count {
                1 => DataType::UI8,
                4 => DataType::UI32,
                _ => DataType::UI16,
            },
        }
    }

    pub(crate) fn category(&self) -> &'static str {
        match self {
            DataType::UI8 | DataType::UI16 | DataType::UI32 => "UI",
            DataType::SI8 | DataType::SI16 | DataType::SI32 => "SI",
            DataType::BF => "BF",
        }
    }

    pub fn is_bitfield(&self) -> bool {
        matches!(self, DataType::BF)
    }
}

/// One contiguous field of the frame. A caller constructs one with
/// [`ChannelDef::new`] and sets the remaining fields directly; the field
/// list can grow with the specification, so literal construction is
/// reserved to chdef:
///
/// ```compile_fail
/// let ch = chdef::ChannelDef {
///     number: 1,
///     name: String::new(),
///     byte_count: 2,
///     data_type: chdef::DataType::UI16,
///     lsb: 1.0,
///     offset: 0.0,
///     unit: String::new(),
///     default_value: None,
/// };
/// ```
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ChannelDef {
    pub number: u32,
    pub name: String,
    pub byte_count: usize,
    pub data_type: DataType,
    pub lsb: f64,
    pub offset: f64,
    pub unit: String,
    /// Optional protocol-spec default value parsed from the CSV. Used by
    /// `ConstChannel::resolve` to fold CSV defaults into TOML-declared
    /// constants when the TOML omits an explicit `value`.
    pub default_value: Option<u32>,
    /// Optional lower / upper bound of the physical value (`min` / `max`
    /// columns). Carried, never applied automatically; see [`Value`].
    pub min: Option<Value>,
    pub max: Option<Value>,
    /// Free text columns carried for consumers that group, annotate or
    /// generate code from the definitions (`section` / `memo` / `var`).
    /// chdef never interprets them.
    pub section: String,
    pub memo: String,
    pub var: String,
    /// How the consumer renders the value (`format` column).
    pub format: DisplayFormat,
    /// The `favorite` flag a consumer uses to pin the channel.
    pub favorite: bool,
}

impl ChannelDef {
    /// A channel with the given identity and width; every other field starts
    /// at its unspecified value (`lsb` 1, `offset` 0, empty strings, no
    /// default) and is set directly:
    ///
    /// ```
    /// let mut ch = chdef::ChannelDef::new(1, 2, chdef::DataType::UI16);
    /// ch.lsb = 0.1;
    /// assert_eq!(ch.value_to_raw(2.0), Some(20));
    /// ```
    pub fn new(number: u32, byte_count: usize, data_type: DataType) -> Self {
        ChannelDef {
            number,
            name: String::new(),
            byte_count,
            data_type,
            lsb: 1.0,
            offset: 0.0,
            unit: String::new(),
            default_value: None,
            min: None,
            max: None,
            section: String::new(),
            memo: String::new(),
            var: String::new(),
            format: DisplayFormat::default(),
            favorite: false,
        }
    }

    /// The lower bound as a physical value, resolved with the current
    /// `lsb` / `offset`; `None` when `min` is unspecified.
    pub fn min_value(&self) -> Option<f64> {
        self.min.as_ref().map(|b| self.resolve_value(b))
    }

    /// The upper bound as a physical value, resolved with the current
    /// `lsb` / `offset`; `None` when `max` is unspecified.
    pub fn max_value(&self) -> Option<f64> {
        self.max.as_ref().map(|b| self.resolve_value(b))
    }

    fn resolve_value(&self, value: &Value) -> f64 {
        match *value {
            Value::Physical(v) => v,
            Value::Raw(raw) => self.raw_to_value_u64(raw),
        }
    }

    /// Whether `value` lies inside the declared range. An unspecified side
    /// is unbounded; NaN is never inside; a swapped range contains nothing.
    /// Conversions never call this — range policy is the caller's.
    pub fn range_contains(&self, value: f64) -> bool {
        !value.is_nan()
            && self.min_value().map_or(true, |lo| value >= lo)
            && self.max_value().map_or(true, |hi| value <= hi)
    }

    /// `value` clamped into the declared range — the explicit way to apply
    /// bounds that no conversion applies on its own. NaN stays NaN; a
    /// swapped range clamps to `max`.
    pub fn clamp_to_range(&self, value: f64) -> f64 {
        let mut v = value;
        if let Some(lo) = self.min_value() {
            if v < lo {
                v = lo;
            }
        }
        if let Some(hi) = self.max_value() {
            if v > hi {
                v = hi;
            }
        }
        v
    }

    pub fn raw_to_value(&self, raw_bytes: &[u8]) -> f64 {
        self.raw_to_value_endian(raw_bytes, Endian::Little)
    }

    pub fn raw_to_value_endian(&self, raw_bytes: &[u8], endian: Endian) -> f64 {
        let b2 = |r: &[u8]| {
            [
                r.first().copied().unwrap_or(0),
                r.get(1).copied().unwrap_or(0),
            ]
        };
        let b4 = |r: &[u8]| {
            [
                r.first().copied().unwrap_or(0),
                r.get(1).copied().unwrap_or(0),
                r.get(2).copied().unwrap_or(0),
                r.get(3).copied().unwrap_or(0),
            ]
        };
        let raw = match (&self.data_type, endian) {
            (DataType::UI8, _) => raw_bytes.first().copied().unwrap_or(0) as f64,
            (DataType::SI8, _) => raw_bytes.first().copied().unwrap_or(0) as i8 as f64,
            (DataType::UI16, Endian::Little) => u16::from_le_bytes(b2(raw_bytes)) as f64,
            (DataType::UI16, Endian::Big) => u16::from_be_bytes(b2(raw_bytes)) as f64,
            (DataType::SI16, Endian::Little) => i16::from_le_bytes(b2(raw_bytes)) as f64,
            (DataType::SI16, Endian::Big) => i16::from_be_bytes(b2(raw_bytes)) as f64,
            (DataType::UI32, Endian::Little) => u32::from_le_bytes(b4(raw_bytes)) as f64,
            (DataType::UI32, Endian::Big) => u32::from_be_bytes(b4(raw_bytes)) as f64,
            (DataType::SI32, Endian::Little) => i32::from_le_bytes(b4(raw_bytes)) as f64,
            (DataType::SI32, Endian::Big) => i32::from_be_bytes(b4(raw_bytes)) as f64,
            (DataType::BF, Endian::Little) => match raw_bytes.len() {
                1 => raw_bytes[0] as f64,
                2 => u16::from_le_bytes(b2(raw_bytes)) as f64,
                _ => u32::from_le_bytes(b4(raw_bytes)) as f64,
            },
            (DataType::BF, Endian::Big) => match raw_bytes.len() {
                1 => raw_bytes[0] as f64,
                2 => u16::from_be_bytes(b2(raw_bytes)) as f64,
                _ => u32::from_be_bytes(b4(raw_bytes)) as f64,
            },
        };
        let lsb = if self.lsb == 0.0 { 1.0 } else { self.lsb };
        raw * lsb + self.offset
    }

    /// Physical value of a raw bit pattern already held as an integer
    /// (`raw × lsb + offset`): the register-shaped counterpart of
    /// [`raw_to_value_endian`], with no byte order involved. An `SI`
    /// channel's pattern is sign-extended from its width; `UI` / `BF` are
    /// unsigned. Bits beyond the width are ignored.
    ///
    /// [`raw_to_value_endian`]: ChannelDef::raw_to_value_endian
    pub fn raw_to_value_u64(&self, raw: u64) -> f64 {
        let bits = self.bits();
        let masked = if bits >= 64 {
            raw
        } else {
            raw & ((1u64 << bits) - 1)
        };
        let signed =
            if self.data_type.category() == "SI" && bits < 64 && (masked >> (bits - 1)) & 1 == 1 {
                (masked | (u64::MAX << bits)) as i64 as f64
            } else {
                masked as f64
            };
        let lsb = if self.lsb == 0.0 { 1.0 } else { self.lsb };
        signed * lsb + self.offset
    }

    /// Width of the channel on the wire in bits (`byte_count × 8`, capped at 64).
    pub fn bits(&self) -> u32 {
        (self.byte_count.min(8) * 8) as u32
    }

    /// Physical value → raw bit pattern of the channel's width:
    /// `clamp(round((value − offset) ÷ lsb))`, rounding half away from zero,
    /// clamped to the signed (`SI`) or unsigned (`UI` / `BF`) range of the
    /// width; a negative result is returned as its two's-complement pattern.
    /// `lsb` 0 counts as 1. `None` when `value` is NaN or infinite.
    pub fn value_to_raw(&self, value: f64) -> Option<u64> {
        let lsb = if self.lsb == 0.0 { 1.0 } else { self.lsb };
        let scaled = ((value - self.offset) / lsb).round();
        if !scaled.is_finite() {
            return None;
        }
        let bits = self.bits();
        let mask = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let raw = if self.data_type.category() == "SI" {
            let bound = 2f64.powi(bits as i32 - 1);
            scaled.clamp(-bound, bound - 1.0) as i64 as u64
        } else {
            scaled.clamp(0.0, 2f64.powi(bits as i32) - 1.0) as u64
        };
        Some(raw & mask)
    }

    /// Physical value → the channel's `byte_count` bytes, little-endian.
    pub fn value_to_bytes(&self, value: f64) -> Option<Vec<u8>> {
        self.value_to_bytes_endian(value, Endian::Little)
    }

    /// Physical value → the channel's `byte_count` bytes in the given byte order.
    pub fn value_to_bytes_endian(&self, value: f64, endian: Endian) -> Option<Vec<u8>> {
        Some(self.raw_to_bytes_endian(self.value_to_raw(value)?, endian))
    }

    /// Raw bit pattern → the channel's `byte_count` bytes in the given byte
    /// order: the storey below [`value_to_bytes_endian`]. No rounding and no
    /// clamp — bits beyond the width are cut, so a caller that wants a
    /// wrapping counter passes the wrapped raw and gets the low bytes.
    ///
    /// [`value_to_bytes_endian`]: ChannelDef::value_to_bytes_endian
    pub fn raw_to_bytes_endian(&self, raw: u64, endian: Endian) -> Vec<u8> {
        let mut bytes = raw.to_le_bytes()[..self.byte_count.min(8)].to_vec();
        if endian == Endian::Big {
            bytes.reverse();
        }
        bytes
    }

    /// The inverse of [`raw_to_bytes_endian`]: read the channel's raw bit
    /// pattern out of `bytes` (at most `byte_count` of them) in the given
    /// byte order.
    ///
    /// [`raw_to_bytes_endian`]: ChannelDef::raw_to_bytes_endian
    pub fn raw_from_bytes_endian(&self, bytes: &[u8], endian: Endian) -> u64 {
        let n = bytes.len().min(self.byte_count).min(8);
        let mut raw = 0u64;
        match endian {
            Endian::Little => {
                for i in (0..n).rev() {
                    raw = (raw << 8) | bytes[i] as u64;
                }
            }
            Endian::Big => {
                for &b in &bytes[..n] {
                    raw = (raw << 8) | b as u64;
                }
            }
        }
        raw
    }

    pub fn format_value(&self, raw_bytes: &[u8]) -> String {
        let val = self.raw_to_value(raw_bytes);
        if self.lsb == 0.0 || self.lsb == 1.0 {
            format!("{}", val as i64)
        } else {
            format!("{:.6}", val)
        }
    }
}

/// One named bit of a `BF` channel. Constructed with [`BitFieldDef::new`];
/// the field list can grow with the specification.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct BitFieldDef {
    pub parent_channel: u32,
    pub bit_number: u8,
    pub name: String,
    /// Optional protocol-spec default of the bit (`0` / `1`); `None` keeps
    /// the default bit of the parent channel.
    pub default_value: Option<u8>,
    /// Free text carried for the consumer (`memo` column); never
    /// interpreted.
    pub memo: String,
}

impl BitFieldDef {
    /// The value of this bit inside the parent channel's raw value:
    /// `(raw >> bit) & 1`.
    pub fn bit_of(&self, raw: u64) -> u8 {
        if self.bit_number >= 64 {
            0
        } else {
            ((raw >> self.bit_number) & 1) as u8
        }
    }

    /// A bit with the given parent channel and position; `name` and
    /// `default_value` start unspecified and are set directly.
    pub fn new(parent_channel: u32, bit_number: u8) -> Self {
        BitFieldDef {
            parent_channel,
            bit_number,
            name: String::new(),
            default_value: None,
            memo: String::new(),
        }
    }
}

/// The Layout stage: channels with duplicates removed, their order fixing
/// every position. Built by [`build_layout`]; the field list can grow with
/// the specification.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ChannelLayout {
    pub channels: Vec<ChannelDef>,
    pub bitfields: Vec<BitFieldDef>,
    /// Byte order of every multi-byte channel of the frame. Not written in
    /// the CSV; the consumer sets it (`Little` when unset).
    pub endian: Endian,
}

/// Build the Layout stage from parsed Rows: duplicates are dropped (the
/// first `number`, and the first `(number, bit)`, win; the parser already
/// reported them as Issues), and the cross-file checks run here — a BF row
/// whose parent is missing or not `BF`, or whose bit is beyond the parent
/// width, is skipped with an Issue. Those Issues carry no row: the CSV rows
/// are gone by this stage.
pub fn build_layout(
    channels: Vec<ChannelDef>,
    bitfields: Vec<BitFieldDef>,
) -> Parsed<ChannelLayout> {
    let mut issues = Vec::new();
    let mut unique_channels: Vec<ChannelDef> = Vec::new();
    for ch in channels {
        if !unique_channels.iter().any(|c| c.number == ch.number) {
            unique_channels.push(ch);
        }
    }
    let mut unique_bitfields: Vec<BitFieldDef> = Vec::new();
    for bf in bitfields {
        let no_row = |code: IssueCode, message: String| Issue {
            code,
            row: None,
            col: None,
            message,
        };
        let parent = unique_channels
            .iter()
            .find(|c| c.number == bf.parent_channel);
        let parent = match parent {
            Some(p) if p.data_type.is_bitfield() => p,
            _ => {
                issues.push(no_row(
                    IssueCode::BfParentNotBitfield,
                    format!(
                        "bit {} of channel {} has no `BF` parent channel; the row was skipped.",
                        bf.bit_number, bf.parent_channel
                    ),
                ));
                continue;
            }
        };
        if (bf.bit_number as u32) >= parent.bits() {
            issues.push(no_row(
                IssueCode::BfBitOutOfRange,
                format!(
                    "bit {} of channel {} is beyond its {}-bit width; the row was skipped.",
                    bf.bit_number,
                    bf.parent_channel,
                    parent.bits()
                ),
            ));
            continue;
        }
        if !unique_bitfields
            .iter()
            .any(|b| b.parent_channel == bf.parent_channel && b.bit_number == bf.bit_number)
        {
            unique_bitfields.push(bf);
        }
    }
    Parsed {
        value: ChannelLayout {
            channels: unique_channels,
            bitfields: unique_bitfields,
            endian: Endian::Little,
        },
        issues,
    }
}

/// One channel of a decoded frame: its definition, the bytes it occupies,
/// and the raw / physical readings.
#[non_exhaustive]
#[derive(Debug)]
pub struct Decoded<'l, 'f> {
    pub channel: &'l ChannelDef,
    pub bytes: &'f [u8],
    pub raw: u64,
    pub value: f64,
}

impl ChannelLayout {
    /// Total data length of the frame: the sum of the channel widths.
    /// Computed on demand, so an edited `byte_count` is never stale.
    pub fn total_bytes(&self) -> usize {
        self.channels.iter().map(|ch| ch.byte_count).sum()
    }

    /// The `layout_exceeds_capacity` Issue when the frame does not fit in
    /// `capacity` bytes, `None` when it does. Nothing calls this on its
    /// own — a consumer with a capacity opts in.
    pub fn check_capacity(&self, capacity: usize) -> Option<Issue> {
        let total = self.total_bytes();
        (total > capacity).then(|| Issue {
            code: IssueCode::LayoutExceedsCapacity,
            row: None,
            col: None,
            message: format!("the frame needs {total} bytes but the capacity is {capacity}."),
        })
    }

    /// Decode a frame: every channel that fits, in row order, with its
    /// bytes and its raw / physical readings under the layout's `endian`.
    /// A channel that overruns a short frame is omitted (never zero-filled),
    /// and so is everything after it — positions are cumulative.
    pub fn decode<'l, 'f>(&'l self, frame: &'f [u8]) -> Vec<Decoded<'l, 'f>> {
        let mut offset = 0;
        let mut decoded = Vec::new();
        for ch in &self.channels {
            let end = offset + ch.byte_count;
            if end > frame.len() {
                break;
            }
            let bytes = &frame[offset..end];
            decoded.push(Decoded {
                channel: ch,
                bytes,
                raw: ch.raw_from_bytes_endian(bytes, self.endian),
                value: ch.raw_to_value_endian(bytes, self.endian),
            });
            offset = end;
        }
        decoded
    }

    /// Encode a frame (`docs/spec/conversion.md` §5): `total_bytes()` bytes
    /// in row order under the layout's `endian`. A channel named in
    /// `values` takes that value — physical converted and clamped per §2,
    /// raw truncated to the width per §3; the last entry for a number wins.
    /// Every other channel takes its default (§4). A value naming an
    /// unknown channel, or one that cannot be converted, is reported as an
    /// Issue and the frame is built without it.
    pub fn encode(&self, values: &[(u32, Value)]) -> Parsed<Vec<u8>> {
        let mut issues = Vec::new();
        for (number, _) in values {
            if !self.channels.iter().any(|c| c.number == *number) {
                issues.push(Issue {
                    code: IssueCode::EncodeUnknownChannel,
                    row: None,
                    col: None,
                    message: format!(
                        "channel {number} is not in the layout; the value was ignored."
                    ),
                });
            }
        }

        let mut frame = Vec::with_capacity(self.total_bytes());
        for ch in &self.channels {
            let given = values
                .iter()
                .rev()
                .find(|(n, _)| *n == ch.number)
                .map(|(_, v)| v);
            let raw = match given {
                Some(Value::Raw(r)) => *r,
                Some(Value::Physical(p)) => match ch.value_to_raw(*p) {
                    Some(raw) => raw,
                    None => {
                        issues.push(Issue {
                            code: IssueCode::EncodeValueInvalid,
                            row: None,
                            col: None,
                            message: format!(
                                "the value for channel {} is not a finite number; its default was used.",
                                ch.number
                            ),
                        });
                        self.default_raw(ch)
                    }
                },
                None => self.default_raw(ch),
            };
            frame.extend_from_slice(&ch.raw_to_bytes_endian(raw, self.endian));
        }
        Parsed {
            value: frame,
            issues,
        }
    }

    /// The effective default raw value of a channel
    /// (`docs/spec/conversion.md` §4): its `default` (0 when unspecified)
    /// with each BF row's default bit folded in — `1` sets, `0` clears,
    /// unspecified keeps the channel default's bit. `None` for a channel
    /// the layout does not have.
    pub fn channel_default(&self, number: u32) -> Option<u64> {
        self.channels
            .iter()
            .find(|c| c.number == number)
            .map(|ch| self.default_raw(ch))
    }

    fn default_raw(&self, ch: &ChannelDef) -> u64 {
        let mut raw = u64::from(ch.default_value.unwrap_or(0));
        if ch.data_type.is_bitfield() {
            for bf in self
                .bitfields
                .iter()
                .filter(|b| b.parent_channel == ch.number && b.bit_number < 64)
            {
                match bf.default_value {
                    Some(1) => raw |= 1 << bf.bit_number,
                    Some(0) => raw &= !(1 << bf.bit_number),
                    _ => {}
                }
            }
        }
        raw
    }

    /// The byte slice of one channel inside `frame`; `None` when the
    /// channel is unknown or overruns the frame.
    pub fn channel_bytes<'f>(&self, number: u32, frame: &'f [u8]) -> Option<&'f [u8]> {
        frame.get(self.channel_offset(number)?..self.channel_end(number)?)
    }

    /// Every channel with its byte offset from the start of the frame, in
    /// row order — the walk that `decode` and `encode` perform, for a
    /// consumer laying out its own view.
    pub fn positions(&self) -> impl Iterator<Item = (usize, &ChannelDef)> {
        self.channels.iter().scan(0usize, |at, ch| {
            let start = *at;
            *at += ch.byte_count;
            Some((start, ch))
        })
    }

    /// Byte offset (from the start of the payload) of the given channel.
    pub fn channel_offset(&self, number: u32) -> Option<usize> {
        let mut offset = 0usize;
        for ch in &self.channels {
            if ch.number == number {
                return Some(offset);
            }
            offset += ch.byte_count;
        }
        None
    }

    /// Byte offset of the end (exclusive) of the given channel.
    pub fn channel_end(&self, number: u32) -> Option<usize> {
        let mut offset = 0usize;
        for ch in &self.channels {
            if ch.number == number {
                return Some(offset + ch.byte_count);
            }
            offset += ch.byte_count;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(byte_count: usize, data_type: DataType, lsb: f64, offset: f64) -> ChannelDef {
        let mut c = ChannelDef::new(1, byte_count, data_type);
        c.name = "t".into();
        c.lsb = lsb;
        c.offset = offset;
        c
    }

    fn named(number: u32, byte_count: usize, data_type: DataType, name: &str) -> ChannelDef {
        let mut c = ChannelDef::new(number, byte_count, data_type);
        c.name = name.into();
        c
    }

    #[test]
    fn value_to_raw_applies_offset_and_lsb() {
        let c = ch(2, DataType::UI16, 0.1, -5.0);
        assert_eq!(c.value_to_raw(7.0), Some(120));
    }

    #[test]
    fn value_to_raw_lsb_zero_is_identity() {
        assert_eq!(ch(2, DataType::UI16, 0.0, 0.0).value_to_raw(42.0), Some(42));
    }

    #[test]
    fn value_to_raw_rounds_half_away_from_zero() {
        let u = ch(1, DataType::UI8, 1.0, 0.0);
        assert_eq!(u.value_to_raw(0.5), Some(1));
        assert_eq!(u.value_to_raw(2.5), Some(3));
        let s = ch(1, DataType::SI8, 1.0, 0.0);
        assert_eq!(s.value_to_raw(-0.5), Some(0xFF));
    }

    #[test]
    fn value_to_raw_negative_becomes_twos_complement_of_width() {
        assert_eq!(
            ch(2, DataType::SI16, 1.0, 0.0).value_to_raw(-2.0),
            Some(0xFFFE)
        );
        assert_eq!(
            ch(4, DataType::SI32, 1.0, 0.0).value_to_raw(-1.0),
            Some(0xFFFF_FFFF)
        );
    }

    #[test]
    fn value_to_raw_clamps_to_width() {
        assert_eq!(
            ch(1, DataType::UI8, 1.0, 0.0).value_to_raw(300.0),
            Some(255)
        );
        assert_eq!(ch(1, DataType::UI8, 1.0, 0.0).value_to_raw(-3.0), Some(0));
        assert_eq!(
            ch(1, DataType::SI8, 1.0, 0.0).value_to_raw(200.0),
            Some(127)
        );
        assert_eq!(
            ch(1, DataType::SI8, 1.0, 0.0).value_to_raw(-200.0),
            Some(0x80)
        );
        assert_eq!(
            ch(2, DataType::BF, 1.0, 0.0).value_to_raw(70000.0),
            Some(0xFFFF)
        );
    }

    #[test]
    fn value_to_raw_supports_64_bit_width() {
        assert_eq!(
            ch(8, DataType::UI16, 1.0, 0.0).value_to_raw(1e19),
            Some(10_000_000_000_000_000_000)
        );
        assert_eq!(
            ch(8, DataType::SI16, 1.0, 0.0).value_to_raw(-1.0),
            Some(u64::MAX)
        );
    }

    #[test]
    fn value_to_raw_rejects_non_finite() {
        let c = ch(2, DataType::UI16, 1.0, 0.0);
        assert_eq!(c.value_to_raw(f64::NAN), None);
        assert_eq!(c.value_to_raw(f64::INFINITY), None);
    }

    #[test]
    fn value_to_bytes_is_inverse_of_raw_to_value() {
        let c = ch(2, DataType::SI16, 0.1, 0.0);
        let bytes = c.value_to_bytes_endian(-12.3, Endian::Big).unwrap();
        assert_eq!(bytes, vec![0xFF, 0x85]);
        assert!((c.raw_to_value_endian(&bytes, Endian::Big) + 12.3).abs() < 1e-9);
        assert_eq!(c.value_to_bytes(-12.3).unwrap(), vec![0x85, 0xFF]);
    }

    #[test]
    fn build_layout_total_bytes() {
        let channels = vec![
            named(1, 4, DataType::UI32, "a"),
            named(2, 2, DataType::UI16, "b"),
            named(3, 1, DataType::UI8, "c"),
        ];
        let layout = build_layout(channels, vec![]).value;
        assert_eq!(layout.total_bytes(), 7);
        assert_eq!(layout.channels.len(), 3);
    }

    #[test]
    fn raw_to_value_identity() {
        let ch = named(1, 2, DataType::UI16, "test");
        assert_eq!(ch.raw_to_value(&[0x2A, 0x00]), 42.0);
    }

    #[test]
    fn raw_to_value_with_lsb_and_offset() {
        let ch = {
            let mut c = named(1, 4, DataType::SI32, "lat");
            c.lsb = 4.19e-08;
            c.offset = 0.0;
            c.unit = "deg".into();
            c
        };
        let raw = 1_000_000i32.to_le_bytes();
        let val = ch.raw_to_value(&raw);
        assert!((val - 0.0419).abs() < 0.001);
    }

    #[test]
    fn raw_to_value_lsb_zero_treated_as_identity() {
        let ch = {
            let mut c = named(1, 2, DataType::UI16, "test");
            c.lsb = 0.0;
            c.offset = 0.0;
            c
        };
        assert_eq!(ch.raw_to_value(&[0x0A, 0x00]), 10.0);
    }

    #[test]
    fn raw_to_value_signed() {
        let ch = {
            let mut c = named(1, 1, DataType::SI8, "temp");
            c.lsb = 1.0;
            c.offset = 0.0;
            c.unit = "℃".into();
            c
        };
        assert_eq!(ch.raw_to_value(&[0xFE]), -2.0); // -2 as i8
    }

    #[test]
    fn format_value_integer_when_lsb_is_one() {
        let ch = named(1, 2, DataType::UI16, "cnt");
        assert_eq!(ch.format_value(&[0x2A, 0x00]), "42");
    }

    #[test]
    fn format_value_decimal_when_lsb_is_fractional() {
        let ch = {
            let mut c = named(1, 4, DataType::SI32, "lat");
            c.lsb = 4.19e-08;
            c.offset = 0.0;
            c.unit = "deg".into();
            c
        };
        let raw = 1_000_000i32.to_le_bytes();
        let formatted = ch.format_value(&raw);
        assert!(formatted.contains("0.0419"), "Got: {}", formatted);
    }

    #[test]
    fn bound_raw_resolves_with_the_current_lsb_and_offset() {
        let mut ch = ChannelDef::new(1, 2, DataType::UI16);
        ch.lsb = 0.5;
        ch.offset = -5.0;
        ch.min = Some(Value::Raw(0x10));
        ch.max = Some(Value::Physical(100.0));

        assert_eq!(ch.min_value(), Some(3.0));
        assert_eq!(ch.max_value(), Some(100.0));

        ch.lsb = 1.0;
        assert_eq!(ch.min_value(), Some(11.0));
    }

    #[test]
    fn bound_raw_sign_extends_for_si_channels() {
        let mut ch = ChannelDef::new(1, 1, DataType::SI8);
        ch.min = Some(Value::Raw(0xFF));

        assert_eq!(ch.min_value(), Some(-1.0));
    }

    #[test]
    fn range_contains_is_true_when_unbounded_and_never_for_nan() {
        let ch = ChannelDef::new(1, 2, DataType::UI16);

        assert!(ch.range_contains(1e9));
        assert!(!ch.range_contains(f64::NAN));
    }

    #[test]
    fn range_queries_use_the_bounds() {
        let mut ch = ChannelDef::new(1, 2, DataType::UI16);
        ch.min = Some(Value::Physical(-40.0));
        ch.max = Some(Value::Physical(120.0));

        assert!(ch.range_contains(0.0));
        assert!(!ch.range_contains(-40.1));
        assert_eq!(ch.clamp_to_range(150.0), 120.0);
        assert_eq!(ch.clamp_to_range(-100.0), -40.0);
        assert_eq!(ch.clamp_to_range(0.5), 0.5);
    }

    #[test]
    fn build_layout_defaults_to_little_endian() {
        assert_eq!(build_layout(vec![], vec![]).value.endian, Endian::Little);
    }

    #[test]
    fn channel_def_new_starts_at_the_unspecified_values() {
        let ch = ChannelDef::new(1, 2, DataType::UI16);

        assert_eq!((ch.number, ch.byte_count), (1, 2));
        assert_eq!(ch.data_type, DataType::UI16);
        assert_eq!((ch.lsb, ch.offset), (1.0, 0.0));
        assert_eq!(ch.default_value, None);
        assert_eq!((ch.min, ch.max), (None, None));
        assert!(ch.name.is_empty() && ch.unit.is_empty());
    }

    #[test]
    fn bit_field_def_new_starts_at_the_unspecified_values() {
        let bf = BitFieldDef::new(2, 3);

        assert_eq!((bf.parent_channel, bf.bit_number), (2, 3));
        assert_eq!(bf.default_value, None);
        assert!(bf.name.is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn endian_serializes_as_a_lowercase_word() {
        assert_eq!(
            serde_json::to_string(&Endian::Little).unwrap(),
            "\"little\""
        );
        assert_eq!(serde_json::to_string(&Endian::Big).unwrap(), "\"big\"");
    }

    fn bf_parent() -> ChannelDef {
        ChannelDef::new(2, 2, DataType::BF)
    }

    #[test]
    fn raw_to_bytes_endian_writes_the_low_bytes_of_the_width() {
        let c = ch(1, DataType::UI8, 1.0, 0.0);
        assert_eq!(c.raw_to_bytes_endian(0x1FF, Endian::Little), vec![0xFF]);
        assert_eq!(c.raw_to_bytes_endian(256, Endian::Little), vec![0x00]);

        let c = ch(2, DataType::UI16, 1.0, 0.0);
        assert_eq!(
            c.raw_to_bytes_endian(0x1234, Endian::Little),
            vec![0x34, 0x12]
        );
        assert_eq!(c.raw_to_bytes_endian(0x1234, Endian::Big), vec![0x12, 0x34]);

        let wrapped = (-2i64) as u64;
        assert_eq!(
            c.raw_to_bytes_endian(wrapped, Endian::Little),
            vec![0xFE, 0xFF]
        );
    }

    #[test]
    fn raw_from_bytes_endian_reads_the_width_back() {
        let c = ch(2, DataType::UI16, 1.0, 0.0);
        assert_eq!(
            c.raw_from_bytes_endian(&[0x34, 0x12], Endian::Little),
            0x1234
        );
        assert_eq!(c.raw_from_bytes_endian(&[0x12, 0x34], Endian::Big), 0x1234);

        let c = ch(4, DataType::UI32, 1.0, 0.0);
        let bytes = c.raw_to_bytes_endian(0xDEADBEEF, Endian::Big);
        assert_eq!(c.raw_from_bytes_endian(&bytes, Endian::Big), 0xDEADBEEF);
    }

    #[test]
    fn build_layout_skips_bf_rows_whose_parent_is_not_a_bitfield() {
        let channels = vec![ChannelDef::new(1, 2, DataType::UI16)];
        let bitfields = vec![bf(0, "on ui16"), BitFieldDef::new(9, 0)];

        let parsed = build_layout(channels, bitfields);

        assert!(parsed.value.bitfields.is_empty());
        assert_eq!(parsed.issues.len(), 2);
        assert!(parsed
            .issues
            .iter()
            .all(|i| i.code == crate::issue::IssueCode::BfParentNotBitfield));
        assert_eq!(parsed.issues[0].row, None);
    }

    #[test]
    fn build_layout_skips_bf_bits_beyond_the_parent_width() {
        let parsed = build_layout(vec![bf_parent()], vec![bf(15, "top"), bf(16, "beyond")]);

        assert_eq!(parsed.value.bitfields.len(), 1);
        assert_eq!(parsed.value.bitfields[0].bit_number, 15);
        assert_eq!(parsed.issues.len(), 1);
        assert_eq!(
            parsed.issues[0].code,
            crate::issue::IssueCode::BfBitOutOfRange
        );
    }

    #[test]
    fn check_capacity_reports_only_when_the_frame_does_not_fit() {
        let layout = build_layout(vec![ch(4, DataType::UI32, 1.0, 0.0)], vec![]).value;

        assert!(layout.check_capacity(4).is_none());
        let issue = layout.check_capacity(3).unwrap();
        assert_eq!(issue.code, crate::issue::IssueCode::LayoutExceedsCapacity);
        assert_eq!(issue.row, None);
    }

    #[test]
    fn decode_slices_the_frame_in_row_order_and_omits_overruns() {
        let mut a = ChannelDef::new(1, 2, DataType::UI16);
        a.lsb = 0.5;
        let b = ChannelDef::new(2, 1, DataType::UI8);
        let layout = build_layout(vec![a, b], vec![]).value;

        let decoded = layout.decode(&[0x0A, 0x00, 0x07]);
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].bytes, &[0x0A, 0x00]);
        assert_eq!(decoded[0].raw, 10);
        assert_eq!(decoded[0].value, 5.0);
        assert_eq!((decoded[1].channel.number, decoded[1].raw), (2, 7));

        let short = layout.decode(&[0x0A, 0x00]);
        assert_eq!(short.len(), 1);
        assert_eq!(short[0].channel.number, 1);
    }

    #[test]
    fn decode_reads_raw_with_the_layout_endian() {
        let mut layout = build_layout(vec![ChannelDef::new(1, 2, DataType::UI16)], vec![]).value;
        layout.endian = Endian::Big;

        let decoded = layout.decode(&[0x12, 0x34]);
        assert_eq!(decoded[0].raw, 0x1234);
        assert_eq!(decoded[0].value, 0x1234 as f64);
    }

    #[test]
    fn channel_bytes_finds_the_slice_of_one_channel() {
        let layout = build_layout(
            vec![
                ChannelDef::new(1, 2, DataType::UI16),
                ChannelDef::new(2, 1, DataType::UI8),
            ],
            vec![],
        )
        .value;

        let frame = [0xAA, 0xBB, 0xCC];
        assert_eq!(layout.channel_bytes(2, &frame), Some(&frame[2..3]));
        assert_eq!(layout.channel_bytes(2, &frame[..2]), None);
        assert_eq!(layout.channel_bytes(9, &frame), None);
    }

    #[test]
    fn bit_of_extracts_the_bit_from_the_parent_raw() {
        let b = BitFieldDef::new(2, 3);
        assert_eq!(b.bit_of(0b0000_1000), 1);
        assert_eq!(b.bit_of(0b1111_0111), 0);
    }

    #[test]
    fn new_channels_start_with_every_carried_column_empty() {
        let c = ChannelDef::new(1, 2, DataType::UI16);

        assert!(c.section.is_empty() && c.memo.is_empty() && c.var.is_empty());
        assert_eq!(c.format, DisplayFormat::Dec);
        assert!(!c.favorite);
    }

    #[test]
    fn display_format_parses_case_insensitively() {
        assert_eq!(DisplayFormat::parse("HEX"), Some(DisplayFormat::Hex));
        assert_eq!(DisplayFormat::parse("dec"), Some(DisplayFormat::Dec));
        assert_eq!(DisplayFormat::parse("octal"), None);
    }

    #[test]
    fn raw_to_value_u64_matches_the_byte_form() {
        let mut c = ch(2, DataType::SI16, 0.1, -5.0);
        c.lsb = 0.1;

        let bytes = c.raw_to_bytes_endian(0xFFFE, Endian::Little);
        assert_eq!(
            c.raw_to_value_u64(0xFFFE),
            c.raw_to_value_endian(&bytes, Endian::Little)
        );
        assert_eq!(c.raw_to_value_u64(0xFFFE), -5.2);
    }

    #[test]
    fn raw_to_value_u64_sign_extends_only_signed_channels() {
        assert_eq!(ch(1, DataType::SI8, 1.0, 0.0).raw_to_value_u64(0xFF), -1.0);
        assert_eq!(ch(1, DataType::UI8, 1.0, 0.0).raw_to_value_u64(0xFF), 255.0);
        assert_eq!(
            ch(2, DataType::BF, 1.0, 0.0).raw_to_value_u64(0xFFFF),
            65535.0
        );
    }

    #[test]
    fn positions_pair_each_channel_with_its_offset() {
        let layout = build_layout(
            vec![
                ChannelDef::new(1, 4, DataType::UI32),
                ChannelDef::new(2, 2, DataType::UI16),
                ChannelDef::new(3, 1, DataType::UI8),
            ],
            vec![],
        )
        .value;

        let seen: Vec<(usize, u32)> = layout.positions().map(|(at, ch)| (at, ch.number)).collect();

        assert_eq!(seen, vec![(0, 1), (4, 2), (6, 3)]);
        assert_eq!(layout.total_bytes(), 7);
    }

    #[test]
    fn value_parse_reads_raw_or_physical_notation() {
        assert_eq!(Value::parse("0x7E"), Some(Value::Raw(0x7E)));
        assert_eq!(Value::parse(" -1.5 "), Some(Value::Physical(-1.5)));
        assert_eq!(Value::parse("abc"), None);
        assert_eq!(Value::parse("0xZZ"), None);
        assert_eq!(Value::parse("NaN"), None);
    }

    #[test]
    fn channel_default_folds_bf_bits_into_the_channel_default() {
        let bit = |b: u8, dv: Option<u8>| {
            let mut x = BitFieldDef::new(2, b);
            x.default_value = dv;
            x
        };

        let mut parent = ChannelDef::new(2, 2, DataType::BF);
        parent.default_value = Some(0x0010);
        let layout = build_layout(
            vec![parent],
            vec![bit(0, Some(1)), bit(2, Some(1)), bit(4, None)],
        )
        .value;
        assert_eq!(layout.channel_default(2), Some(0x0015));

        let mut parent = ChannelDef::new(2, 2, DataType::BF);
        parent.default_value = Some(0x00FF);
        let layout = build_layout(vec![parent], vec![bit(0, Some(0)), bit(4, Some(0))]).value;
        assert_eq!(layout.channel_default(2), Some(0x00EE));
        assert_eq!(layout.channel_default(9), None);
    }

    #[test]
    fn encode_fills_unnamed_channels_with_their_defaults() {
        let mut a = ChannelDef::new(1, 1, DataType::UI8);
        a.default_value = Some(0x7E);
        let b = ChannelDef::new(2, 2, DataType::UI16);
        let layout = build_layout(vec![a, b], vec![]).value;

        let out = layout.encode(&[]);

        assert!(out.issues.is_empty());
        assert_eq!(out.value, vec![0x7E, 0x00, 0x00]);
    }

    #[test]
    fn encode_converts_physical_and_truncates_raw_values() {
        let mut a = ChannelDef::new(1, 2, DataType::UI16);
        a.lsb = 0.5;
        a.offset = -5.0;
        let b = ChannelDef::new(2, 1, DataType::UI8);
        let layout = build_layout(vec![a, b], vec![]).value;

        let out = layout.encode(&[(1, Value::Physical(5.0)), (2, Value::Raw(0x1FF))]);

        assert_eq!(out.value, vec![20, 0, 0xFF]);
        assert!(out.issues.is_empty());
    }

    #[test]
    fn encode_respects_the_layout_endian() {
        let mut layout = build_layout(vec![ChannelDef::new(1, 2, DataType::UI16)], vec![]).value;
        layout.endian = Endian::Big;

        assert_eq!(
            layout.encode(&[(1, Value::Raw(0x1234))]).value,
            vec![0x12, 0x34]
        );
    }

    #[test]
    fn encode_reports_values_it_cannot_place() {
        let layout = build_layout(vec![ChannelDef::new(1, 2, DataType::UI16)], vec![]).value;

        let out = layout.encode(&[(9, Value::Raw(1)), (1, Value::Physical(f64::NAN))]);

        assert_eq!(out.value, vec![0, 0]);
        let codes: Vec<_> = out.issues.iter().map(|i| i.code).collect();
        assert_eq!(
            codes,
            vec![
                crate::issue::IssueCode::EncodeUnknownChannel,
                crate::issue::IssueCode::EncodeValueInvalid,
            ]
        );
    }

    #[test]
    fn encode_lets_the_last_value_for_a_channel_win() {
        let layout = build_layout(vec![ChannelDef::new(1, 1, DataType::UI8)], vec![]).value;

        let out = layout.encode(&[(1, Value::Raw(1)), (1, Value::Raw(2))]);

        assert_eq!(out.value, vec![2]);
    }

    #[test]
    fn decode_reads_back_what_encode_wrote() {
        let mut a = ChannelDef::new(1, 2, DataType::SI16);
        a.lsb = 0.1;
        let b = ChannelDef::new(2, 4, DataType::UI32);
        let layout = build_layout(vec![a, b], vec![]).value;

        let frame = layout
            .encode(&[(1, Value::Physical(-1.5)), (2, Value::Raw(0xDEADBEEF))])
            .value;
        let decoded = layout.decode(&frame);

        assert_eq!(decoded[0].value, -1.5);
        assert_eq!(decoded[1].raw, 0xDEADBEEF);
    }

    #[test]
    fn build_layout_keeps_the_first_of_duplicate_numbers() {
        let dup =
            |number: u32, byte_count: usize| ChannelDef::new(number, byte_count, DataType::UI16);

        let layout = build_layout(vec![dup(1, 2), dup(1, 4), dup(2, 1)], vec![]).value;

        assert_eq!(layout.channels.len(), 2);
        assert_eq!(layout.channels[0].byte_count, 2);
        assert_eq!(layout.total_bytes(), 3);
    }

    fn bf(bit: u8, name: &str) -> BitFieldDef {
        let mut b = BitFieldDef::new(2, bit);
        b.name = name.into();
        b
    }

    #[test]
    fn build_layout_keeps_the_first_of_duplicate_bits() {
        let layout = build_layout(
            vec![bf_parent()],
            vec![bf(0, "first"), bf(0, "second"), bf(1, "other")],
        )
        .value;

        assert_eq!(layout.bitfields.len(), 2);
        assert_eq!(layout.bitfields[0].name, "first");
    }
}
