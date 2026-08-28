use crate::derived::{Derivation, DerivedRecipe};
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

/// The same order, as the core states it: the core carries the arithmetic
/// and must not carry chdef's serde derives, so the two enums stay
/// separate and meet here.
fn core_endian(endian: Endian) -> chdef_core::Endian {
    match endian {
        Endian::Little => chdef_core::Endian::Little,
        Endian::Big => chdef_core::Endian::Big,
    }
}

/// Which of a channel's two readings the consumer shows: the physical
/// value, or the raw bit pattern. The `format` column spells these `DEC`
/// and `HEX` — the base each is conventionally printed in — but what the
/// column selects is the value, not the base
/// (`docs/spec/conversion.md` §7). Selecting one changes no conversion:
/// both readings are always available.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ValueDisplay {
    #[default]
    Physical,
    Raw,
}

impl ValueDisplay {
    /// Read the `format` column: `DEC` / `HEX`, case-insensitively and
    /// after trimming; `None` for anything else.
    pub fn parse(s: &str) -> Option<ValueDisplay> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("dec") {
            Some(ValueDisplay::Physical)
        } else if s.eq_ignore_ascii_case("hex") {
            Some(ValueDisplay::Raw)
        } else {
            None
        }
    }

    /// The spelling the `format` column uses for this.
    pub fn as_str(self) -> &'static str {
        match self {
            ValueDisplay::Physical => "DEC",
            ValueDisplay::Raw => "HEX",
        }
    }
}

impl std::fmt::Display for ValueDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

/// How a channel's bytes are interpreted. The width is never part of this:
/// it is the channel's `bytes` (`docs/spec/layout.md` §6). Further
/// interpretations may appear, so matches need a catch-all arm.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Unsigned integer.
    #[default]
    UI,
    /// Two's-complement signed integer, sign-extended at the channel width.
    SI,
    /// A bag of bits, unsigned when read as a number.
    BF,
}

/// Who decides a channel's value (`docs/spec/format.md` §5).
///
/// A mark, not a behaviour: chdef carries it and acts on none of it, and
/// [`ChannelLayout::encode`] produces the same bytes whatever it says
/// (ADR-0025). Further kinds may appear, so matches need a catch-all arm.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ChannelKind {
    /// The caller supplies the value, or the channel takes its `default`.
    #[default]
    Plain,
    /// The value is the `default` and does not change frame to frame.
    Const,
    /// The caller supplies a number that advances every frame. chdef never
    /// advances it: a counter belongs to the line that sends the frames,
    /// and one definition may be shared by several.
    Counter,
    /// chdef computes it from the rest of the frame, by the recipe in the
    /// `derived` column. Filled by [`ChannelLayout::seal`], never by
    /// [`ChannelLayout::encode`].
    Derived,
}

impl ChannelKind {
    /// Read the `kind` column, trimmed and case-insensitively; `None` for
    /// a value this chdef does not know.
    pub fn parse(s: &str) -> Option<ChannelKind> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("plain") {
            Some(ChannelKind::Plain)
        } else if s.eq_ignore_ascii_case("const") {
            Some(ChannelKind::Const)
        } else if s.eq_ignore_ascii_case("counter") {
            Some(ChannelKind::Counter)
        } else if s.eq_ignore_ascii_case("derived") {
            Some(ChannelKind::Derived)
        } else {
            None
        }
    }

    /// The spelling the `kind` column uses for this.
    pub fn as_str(self) -> &'static str {
        match self {
            ChannelKind::Plain => "plain",
            ChannelKind::Const => "const",
            ChannelKind::Counter => "counter",
            ChannelKind::Derived => "derived",
        }
    }
}

impl std::fmt::Display for ChannelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl DataType {
    /// The two-letter tag the `type` column spells this with.
    pub fn as_str(self) -> &'static str {
        match self {
            DataType::UI => "UI",
            DataType::SI => "SI",
            DataType::BF => "BF",
        }
    }

    pub fn is_bitfield(self) -> bool {
        matches!(self, DataType::BF)
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
///     data_type: chdef::DataType::UI,
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
    /// Optional protocol-spec default of the channel, as a raw value of the
    /// channel's own width (`docs/spec/conversion.md` §4). `None` when the
    /// `default` column is empty.
    pub default_value: Option<u64>,
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
    /// Which reading the consumer shows (`format` column); never affects a
    /// conversion. See [`ValueDisplay`].
    pub format: ValueDisplay,
    /// The `favorite` flag a consumer uses to pin the channel.
    pub favorite: bool,
    /// Who decides this channel value (`docs/spec/format.md` §5).
    /// Carried, never acted on.
    pub kind: ChannelKind,
    /// How a `derived` channel is computed (`docs/spec/format.md` §6);
    /// `None` for every other kind, and for a recipe chdef cannot read.
    pub derived: Option<DerivedRecipe>,
}

impl ChannelDef {
    /// A channel with the given identity and width; every other field starts
    /// at its unspecified value (`lsb` 1, `offset` 0, empty strings, no
    /// default) and is set directly:
    ///
    /// ```
    /// let mut ch = chdef::ChannelDef::new(1, 2, chdef::DataType::UI);
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
            format: ValueDisplay::default(),
            favorite: false,
            kind: ChannelKind::default(),
            derived: None,
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

    /// The finding when `value` lies outside the declared range, naming
    /// the bound it crossed (`docs/spec/conversion.md` §8); `None` when it
    /// is inside, or when neither side is declared.
    pub(crate) fn out_of_range(&self, value: f64) -> Option<Issue> {
        let crossed = match (self.min_value(), self.max_value()) {
            (Some(low), _) if value < low => low,
            (_, Some(high)) if value > high => high,
            _ => return None,
        };
        Some(
            Issue::new(
                IssueCode::ValueOutOfRange,
                format!(
                    "the value {value} for channel {} is outside its declared range; the bound it crosses is {crossed}.",
                    self.number
                ),
            )
            .about_channel(self.number)
            .found(value.to_string())
            .used(crossed.to_string()),
        )
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

    /// Physical value of the channel's bytes in the given byte order
    /// (`docs/spec/conversion.md` §1). The channel's whole width is read;
    /// bytes past it, and bytes it is missing, are ignored.
    pub fn raw_to_value_endian(&self, raw_bytes: &[u8], endian: Endian) -> f64 {
        self.raw_to_value_u64(self.raw_from_bytes_endian(raw_bytes, endian))
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
        let signed = if self.data_type == DataType::SI && (masked >> (bits - 1)) & 1 == 1 {
            let extended = if bits >= 64 {
                masked
            } else {
                masked | (u64::MAX << bits)
            };
            extended as i64 as f64
        } else {
            masked as f64
        };
        let lsb = if self.lsb == 0.0 { 1.0 } else { self.lsb };
        signed * lsb + self.offset
    }

    /// Width of the channel on the wire in bytes: `byte_count` held to the
    /// 1–8 the specification allows (`docs/spec/format.md` §3 clamps the
    /// column the same way). Every conversion and every position measures
    /// the channel with this, so a `byte_count` outside the range is read
    /// as the nearest legal width rather than panicking.
    pub fn width(&self) -> usize {
        self.byte_count.clamp(1, 8)
    }

    /// Width of the channel on the wire in bits (`width() × 8`), 8 to 64.
    pub fn bits(&self) -> u32 {
        (self.width() * 8) as u32
    }

    /// Physical value → raw bit pattern of the channel's width:
    /// `clamp(round((value − offset) ÷ lsb))`, rounding half away from zero,
    /// clamped to the signed (`SI`) or unsigned (`UI` / `BF`) range of the
    /// width; a negative result is returned as its two's-complement pattern.
    /// `lsb` 0 counts as 1. `None` when `value` is NaN or infinite.
    pub fn value_to_raw(&self, value: f64) -> Option<u64> {
        self.scale_to_raw(value).map(|(raw, _)| raw)
    }

    /// Whether the channel width can hold `value` as given
    /// (`docs/spec/conversion.md` §2) — the ask the primitives answer,
    /// since they return a number and not findings. `false` for a value
    /// that cannot be converted at all.
    ///
    /// At a 64-bit width the answer is bounded by the f64 limit of §1 and
    /// errs towards saying a value fits.
    pub fn fits_width(&self, value: f64) -> bool {
        matches!(self.scale_to_raw(value), Some((_, false)))
    }

    /// The raw value, and whether the width had to clamp it.
    fn scale_to_raw(&self, value: f64) -> Option<(u64, bool)> {
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

        // The bounds in the same terms the clamp below uses. Beyond 53
        // bits f64 cannot hold them exactly, and rounding outwards is the
        // safe direction: a clamp may go unreported, but one is never
        // claimed that did not happen.
        let (low, high) = if self.data_type == DataType::SI {
            let bound = 2f64.powi(bits as i32 - 1);
            (-bound, bound - 1.0)
        } else {
            (0.0, 2f64.powi(bits as i32) - 1.0)
        };
        let clamped = scaled < low || scaled > high;
        // The clamp happens in integer space: a bound of the form 2^n - 1 is
        // not representable in f64 beyond 53 bits, and rounding it up to 2^n
        // would leave the mask below to erase the value.
        let raw = if self.data_type == DataType::SI {
            let bound = 2f64.powi(bits as i32 - 1);
            let (min, max) = if bits >= 64 {
                (i64::MIN, i64::MAX)
            } else {
                (-(1i64 << (bits - 1)), (1i64 << (bits - 1)) - 1)
            };
            (scaled.clamp(-bound, bound) as i64).clamp(min, max) as u64
        } else {
            (scaled.max(0.0) as u64).min(mask)
        };
        Some((raw & mask, clamped))
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
        let mut bytes = vec![0u8; self.width()];
        let written = chdef_core::write_raw(&mut bytes, self.width(), core_endian(endian), raw);
        debug_assert!(written);
        bytes
    }

    /// The inverse of [`raw_to_bytes_endian`]: read the channel's raw bit
    /// pattern out of `bytes` (at most `byte_count` of them) in the given
    /// byte order.
    ///
    /// [`raw_to_bytes_endian`]: ChannelDef::raw_to_bytes_endian
    pub fn raw_from_bytes_endian(&self, bytes: &[u8], endian: Endian) -> u64 {
        chdef_core::read_raw(bytes, self.width(), core_endian(endian))
    }

    /// The reading this channel's `format` column selects, as a [`Value`]
    /// carrying its own notation. Deciding which value to show is chdef's;
    /// turning it into text is the consumer's, and [`render`] is one way.
    ///
    /// [`render`]: ChannelDef::render
    pub fn displayed_value(&self, raw: u64) -> Value {
        match self.format {
            ValueDisplay::Physical => Value::Physical(self.raw_to_value_u64(raw)),
            ValueDisplay::Raw => Value::Raw(raw),
        }
    }

    /// A default rendering of [`displayed_value`]: the physical value with
    /// the channel's `unit` after it when there is one, or the raw value in
    /// hexadecimal padded to the channel's width. A consumer that wants its
    /// own digit counts, separators or colours renders the `Value` itself —
    /// presentation is not chdef's to own
    /// (`docs/spec/README.md`, "Out of scope").
    ///
    /// [`displayed_value`]: ChannelDef::displayed_value
    pub fn render(&self, raw: u64) -> String {
        match self.displayed_value(raw) {
            Value::Raw(r) => format!("0x{r:0width$X}", width = self.width() * 2),
            Value::Physical(v) if self.unit.is_empty() => format!("{v}"),
            Value::Physical(v) => format!("{v} {}", self.unit),
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
    /// The maximum byte count of the data part, when the consumer has one.
    /// Not written in the CSV, and never checked unless
    /// [`limits_exceeded`](ChannelLayout::limits_exceeded) is called.
    pub capacity: Option<usize>,
    /// The maximum number of channels the port accepts, when the consumer
    /// has one — the limit a byte count cannot express. Same terms as
    /// [`capacity`](ChannelLayout::capacity).
    pub channel_capacity: Option<usize>,
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
        let parent = unique_channels
            .iter()
            .find(|c| c.number == bf.parent_channel);
        let parent = match parent {
            Some(p) if p.data_type.is_bitfield() => p,
            _ => {
                issues.push(
                    Issue::new(
                        IssueCode::BfParentNotBitfield,
                        format!(
                            "bit {} of channel {} has no `BF` parent channel; the row was skipped.",
                            bf.bit_number, bf.parent_channel
                        ),
                    )
                    .about_bit(bf.parent_channel, bf.bit_number),
                );
                continue;
            }
        };
        if (bf.bit_number as u32) >= parent.bits() {
            issues.push(
                Issue::new(
                    IssueCode::BfBitOutOfRange,
                    format!(
                        "bit {} of channel {} is beyond its {}-bit width; the row was skipped.",
                        bf.bit_number,
                        bf.parent_channel,
                        parent.bits()
                    ),
                )
                .about_bit(bf.parent_channel, bf.bit_number)
                .found(bf.bit_number.to_string())
                .used(parent.bits().to_string()),
            );
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
            capacity: None,
            channel_capacity: None,
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
    /// Every bit the layout defines, of every channel; `bits()` picks out
    /// this channel's.
    bitfields: &'l [BitFieldDef],
}

impl<'l> Decoded<'l, '_> {
    /// The named bits of this channel and whether each is set
    /// (`docs/spec/conversion.md` §6). Empty for a channel with no bits
    /// named, whatever its type.
    pub fn bits(&self) -> impl Iterator<Item = (&'l BitFieldDef, bool)> {
        let (number, raw, defs) = (self.channel.number, self.raw, self.bitfields);
        defs.iter()
            .filter(move |b| b.parent_channel == number)
            .map(move |b| (b, b.bit_of(raw) == 1))
    }
}

impl ChannelLayout {
    /// Total data length of the frame: the sum of the channel widths.
    /// Computed on demand, so an edited `byte_count` is never stale.
    pub fn total_bytes(&self) -> usize {
        self.channels.iter().map(|ch| ch.width()).sum()
    }

    /// State the maximum byte count of the data part
    /// (`docs/spec/layout.md` §5), so the layout carries what it is
    /// measured against instead of the consumer holding it alongside.
    pub fn with_capacity(mut self, capacity: usize) -> ChannelLayout {
        self.capacity = Some(capacity);
        self
    }

    /// State the maximum number of channels the port accepts
    /// (`docs/spec/layout.md` §5), for
    /// [`limits_exceeded`](ChannelLayout::limits_exceeded).
    pub fn with_channel_capacity(mut self, channels: usize) -> ChannelLayout {
        self.channel_capacity = Some(channels);
        self
    }

    /// The `layout_exceeds_capacity` Issue when the frame does not fit the
    /// layout's `capacity`, `None` when it fits or when there is no
    /// capacity. Nothing calls this on its own — a consumer opts in.
    pub fn limits_exceeded(&self) -> Vec<Issue> {
        let mut issues = Vec::new();

        if let Some(capacity) = self.capacity {
            let total = self.total_bytes();
            if total > capacity {
                issues.push(
                    Issue::new(
                        IssueCode::LayoutExceedsCapacity,
                        format!("the frame needs {total} bytes but the capacity is {capacity}."),
                    )
                    .found(total.to_string())
                    .used(capacity.to_string()),
                );
            }
        }

        if let Some(capacity) = self.channel_capacity {
            let count = self.channels.len();
            if count > capacity {
                issues.push(
                    Issue::new(
                        IssueCode::LayoutExceedsChannelCapacity,
                        format!("the layout has {count} channels but the port accepts {capacity}."),
                    )
                    .found(count.to_string())
                    .used(capacity.to_string()),
                );
            }
        }

        issues
    }

    /// Fill every derived channel of `frame` (`docs/spec/format.md` §6),
    /// in layout order.
    ///
    /// A frame is sealed once, after every other value is in place,
    /// because a recipe reads the bytes as they will be sent. `encode`
    /// never does this: sealing is a call of its own (ADR-0029).
    ///
    /// Returns what could not be done — a frame too short to hold the
    /// channel, or a recipe covering a channel the layout does not have.
    /// Nothing is written for a channel that is reported.
    pub fn seal(&self, frame: &mut [u8]) -> Vec<Issue> {
        let mut issues = Vec::new();
        for (at, ch) in self.positions() {
            let Some(recipe) = ch.derived.as_ref() else {
                continue;
            };
            match self.derived_value(ch, recipe, frame) {
                Ok(value) => {
                    let bytes = ch.raw_to_bytes_endian(value, self.endian);
                    frame[at..at + ch.width()].copy_from_slice(&bytes);
                }
                Err(issue) => issues.push(issue),
            }
        }
        issues
    }

    /// The bytes a derived channel's recipe covers, in the order it
    /// covers them (`docs/spec/format.md` §6); `None` when the channel is
    /// not derived, its recipe was unreadable, or the frame is too short.
    ///
    /// This is the storey below [`seal`](ChannelLayout::seal): a device
    /// whose checksum chdef does not compute still says which bytes it
    /// covers, so a caller runs its own over exactly those and writes the
    /// result with [`encode`](ChannelLayout::encode).
    pub fn covered_bytes(&self, number: u32, frame: &[u8]) -> Option<Vec<u8>> {
        let ch = self.channels.iter().find(|c| c.number == number)?;
        let recipe = ch.derived.as_ref()?;
        let mut covered = Vec::new();
        for (low, high) in &recipe.spans {
            for over in *low..=*high {
                let (at, def) = self.positions().find(|(_, c)| c.number == over)?;
                let stop = at + def.width();
                if frame.len() < stop {
                    return None;
                }
                covered.extend_from_slice(&frame[at..stop]);
            }
        }
        Some(covered)
    }

    /// Which derived channels of `frame` disagree with their recipe
    /// (`docs/spec/format.md` §6) — the check a receiver makes.
    ///
    /// `found` is the value the frame holds and `used` the one the recipe
    /// computes, both in hexadecimal at the channel's width. Nothing is
    /// changed and nothing is remembered.
    pub fn derived_mismatches(&self, frame: &[u8]) -> Vec<Issue> {
        let mut issues = Vec::new();
        for (at, ch) in self.positions() {
            let Some(recipe) = ch.derived.as_ref() else {
                continue;
            };
            let width = ch.width();
            match self.derived_value(ch, recipe, frame) {
                Err(issue) => issues.push(issue),
                Ok(expected) => {
                    let stored = ch.raw_from_bytes_endian(&frame[at..at + width], self.endian);
                    if stored != expected {
                        let digits = width * 2;
                        issues.push(
                            Issue::new(
                                IssueCode::DerivedMismatch,
                                format!(
                                    "channel {} holds 0x{stored:0digits$X} but its recipe computes 0x{expected:0digits$X}.",
                                    ch.number
                                ),
                            )
                            .about_channel(ch.number)
                            .found(format!("0x{stored:0digits$X}"))
                            .used(format!("0x{expected:0digits$X}")),
                        );
                    }
                }
            }
        }
        issues
    }

    /// The value a recipe computes over `frame`, or why it cannot.
    fn derived_value(
        &self,
        ch: &ChannelDef,
        recipe: &DerivedRecipe,
        frame: &[u8],
    ) -> Result<u64, Issue> {
        let end = self
            .channel_offset(ch.number)
            .map(|at| at + ch.width())
            .unwrap_or(0);
        if frame.len() < end {
            return Err(Issue::new(
                IssueCode::DerivedUnknownChannel,
                format!(
                    "the frame is {} bytes, too short to hold channel {}.",
                    frame.len(),
                    ch.number
                ),
            )
            .about_channel(ch.number)
            .found(frame.len().to_string())
            .used(end.to_string()));
        }

        let mut covered = Vec::new();
        for (low, high) in &recipe.spans {
            for number in *low..=*high {
                let Some((at, over)) = self.positions().find(|(_, c)| c.number == number) else {
                    return Err(Issue::new(
                        IssueCode::DerivedUnknownChannel,
                        format!(
                            "the recipe of channel {} covers channel {number}, which the layout does not have.",
                            ch.number
                        ),
                    )
                    .about_channel(ch.number)
                    .found(number.to_string()));
                };
                let stop = at + over.width();
                if frame.len() < stop {
                    return Err(Issue::new(
                        IssueCode::DerivedUnknownChannel,
                        format!(
                            "the frame is {} bytes, too short to cover channel {number}.",
                            frame.len()
                        ),
                    )
                    .about_channel(ch.number)
                    .found(frame.len().to_string())
                    .used(stop.to_string()));
                }
                covered.extend_from_slice(&frame[at..stop]);
            }
        }
        match &recipe.derivation {
            Derivation::Crc(crc) => Ok(crc.of(&covered)),
            other => Err(Issue::new(
                IssueCode::DerivedUnknownRecipe,
                format!(
                    "the recipe of channel {} is {other:?}, which this chdef does not compute; covered_bytes hands over the bytes it covers.",
                    ch.number
                ),
            )
            .about_channel(ch.number)
            .found(match other {
                Derivation::Unknown(name) => name.clone(),
                _ => String::new(),
            })),
        }
    }

    /// Which of `values` fall outside their channel's declared range
    /// (`docs/spec/conversion.md` §8), as Issue `value_out_of_range`.
    ///
    /// Nothing is changed and nothing is remembered: this is the question
    /// asked before encoding, and `encode` behaves the same whether it was
    /// asked or not. A value naming a channel the layout does not have is
    /// not a range finding — `encode` reports that one.
    pub fn values_out_of_range(&self, values: &[(u32, Value)]) -> Vec<Issue> {
        values
            .iter()
            .filter_map(|(number, value)| {
                let ch = self.channels.iter().find(|c| c.number == *number)?;
                let physical = match *value {
                    Value::Physical(v) => v,
                    Value::Raw(r) => ch.raw_to_value_u64(r),
                };
                ch.out_of_range(physical)
            })
            .collect()
    }

    /// Which of `readings` fall outside their channel's declared range —
    /// the same question as [`values_out_of_range`](ChannelLayout::values_out_of_range),
    /// asked of a frame that has arrived.
    pub fn readings_out_of_range(&self, readings: &[Decoded<'_, '_>]) -> Vec<Issue> {
        readings
            .iter()
            .filter_map(|reading| reading.channel.out_of_range(reading.value))
            .collect()
    }

    /// Decode a frame: every channel that fits, in row order, with its
    /// bytes and its raw / physical readings under the layout's `endian`.
    /// A channel that overruns a short frame is omitted (never zero-filled),
    /// and so is everything after it — positions are cumulative.
    pub fn decode<'l, 'f>(&'l self, frame: &'f [u8]) -> Vec<Decoded<'l, 'f>> {
        let mut offset = 0;
        let mut decoded = Vec::new();
        for ch in &self.channels {
            let end = offset + ch.width();
            if end > frame.len() {
                break;
            }
            let bytes = &frame[offset..end];
            decoded.push(Decoded {
                channel: ch,
                bytes,
                raw: ch.raw_from_bytes_endian(bytes, self.endian),
                value: ch.raw_to_value_endian(bytes, self.endian),
                bitfields: &self.bitfields,
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
                issues.push(
                    Issue::new(
                        IssueCode::EncodeUnknownChannel,
                        format!("channel {number} is not in the layout; the value was ignored."),
                    )
                    .about_channel(*number),
                );
            }
        }

        let mut frame = Vec::with_capacity(self.total_bytes());
        for ch in &self.channels {
            let given = values
                .iter()
                .rev()
                .find(|(n, _)| *n == ch.number)
                .map(|(_, v)| v);
            let bits = ch.bits();
            let raw = match given {
                Some(Value::Raw(r)) => {
                    if bits < 64 && r >> bits != 0 {
                        issues.push(
                            Issue::new(
                                IssueCode::RawOutOfRange,
                                format!(
                                    "the raw value 0x{r:X} for channel {} exceeds its {bits}-bit width; the low bits were used.",
                                    ch.number
                                ),
                            )
                            .about_channel(ch.number)
                            .found(format!("0x{r:X}"))
                            .used(format!("0x{:X}", r & ((1u64 << bits) - 1))),
                        );
                    }
                    *r
                }
                Some(Value::Physical(p)) => match ch.scale_to_raw(*p) {
                    Some((raw, clamped)) => {
                        if clamped {
                            issues.push(
                                Issue::new(
                                    IssueCode::EncodeValueClamped,
                                    format!(
                                        "the value {p} for channel {} does not fit its {}-bit width; {} was written.",
                                        ch.number,
                                        ch.bits(),
                                        ch.raw_to_value_u64(raw)
                                    ),
                                )
                                .about_channel(ch.number)
                                .found(p.to_string())
                                .used(ch.raw_to_value_u64(raw).to_string()),
                            );
                        }
                        raw
                    }
                    None => {
                        issues.push(
                            Issue::new(
                                IssueCode::EncodeValueInvalid,
                                format!(
                                    "the value for channel {} is not a finite number; its default was used.",
                                    ch.number
                                ),
                            )
                            .about_channel(ch.number)
                            .found(p.to_string()),
                        );
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
        let mut raw = ch.default_value.unwrap_or(0);
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
            *at += ch.width();
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
            offset += ch.width();
        }
        None
    }

    /// Byte offset of the end (exclusive) of the given channel.
    pub fn channel_end(&self, number: u32) -> Option<usize> {
        let mut offset = 0usize;
        for ch in &self.channels {
            if ch.number == number {
                return Some(offset + ch.width());
            }
            offset += ch.width();
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
        let c = ch(2, DataType::UI, 0.1, -5.0);
        assert_eq!(c.value_to_raw(7.0), Some(120));
    }

    #[test]
    fn value_to_raw_lsb_zero_is_identity() {
        assert_eq!(ch(2, DataType::UI, 0.0, 0.0).value_to_raw(42.0), Some(42));
    }

    #[test]
    fn value_to_raw_rounds_half_away_from_zero() {
        let u = ch(1, DataType::UI, 1.0, 0.0);
        assert_eq!(u.value_to_raw(0.5), Some(1));
        assert_eq!(u.value_to_raw(2.5), Some(3));
        let s = ch(1, DataType::SI, 1.0, 0.0);
        assert_eq!(s.value_to_raw(-0.5), Some(0xFF));
    }

    #[test]
    fn value_to_raw_negative_becomes_twos_complement_of_width() {
        assert_eq!(
            ch(2, DataType::SI, 1.0, 0.0).value_to_raw(-2.0),
            Some(0xFFFE)
        );
        assert_eq!(
            ch(4, DataType::SI, 1.0, 0.0).value_to_raw(-1.0),
            Some(0xFFFF_FFFF)
        );
    }

    #[test]
    fn value_to_raw_clamps_to_width() {
        assert_eq!(ch(1, DataType::UI, 1.0, 0.0).value_to_raw(300.0), Some(255));
        assert_eq!(ch(1, DataType::UI, 1.0, 0.0).value_to_raw(-3.0), Some(0));
        assert_eq!(ch(1, DataType::SI, 1.0, 0.0).value_to_raw(200.0), Some(127));
        assert_eq!(
            ch(1, DataType::SI, 1.0, 0.0).value_to_raw(-200.0),
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
            ch(8, DataType::UI, 1.0, 0.0).value_to_raw(1e19),
            Some(10_000_000_000_000_000_000)
        );
        assert_eq!(
            ch(8, DataType::SI, 1.0, 0.0).value_to_raw(-1.0),
            Some(u64::MAX)
        );
    }

    #[test]
    fn value_to_raw_rejects_non_finite() {
        let c = ch(2, DataType::UI, 1.0, 0.0);
        assert_eq!(c.value_to_raw(f64::NAN), None);
        assert_eq!(c.value_to_raw(f64::INFINITY), None);
    }

    #[test]
    fn value_to_bytes_is_inverse_of_raw_to_value() {
        let c = ch(2, DataType::SI, 0.1, 0.0);
        let bytes = c.value_to_bytes_endian(-12.3, Endian::Big).unwrap();
        assert_eq!(bytes, vec![0xFF, 0x85]);
        assert!((c.raw_to_value_endian(&bytes, Endian::Big) + 12.3).abs() < 1e-9);
        assert_eq!(c.value_to_bytes(-12.3).unwrap(), vec![0x85, 0xFF]);
    }

    #[test]
    fn build_layout_total_bytes() {
        let channels = vec![
            named(1, 4, DataType::UI, "a"),
            named(2, 2, DataType::UI, "b"),
            named(3, 1, DataType::UI, "c"),
        ];
        let layout = build_layout(channels, vec![]).value;
        assert_eq!(layout.total_bytes(), 7);
        assert_eq!(layout.channels.len(), 3);
    }

    #[test]
    fn raw_to_value_identity() {
        let ch = named(1, 2, DataType::UI, "test");
        assert_eq!(ch.raw_to_value(&[0x2A, 0x00]), 42.0);
    }

    #[test]
    fn raw_to_value_with_lsb_and_offset() {
        let ch = {
            let mut c = named(1, 4, DataType::SI, "lat");
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
            let mut c = named(1, 2, DataType::UI, "test");
            c.lsb = 0.0;
            c.offset = 0.0;
            c
        };
        assert_eq!(ch.raw_to_value(&[0x0A, 0x00]), 10.0);
    }

    #[test]
    fn raw_to_value_signed() {
        let ch = {
            let mut c = named(1, 1, DataType::SI, "temp");
            c.lsb = 1.0;
            c.offset = 0.0;
            c.unit = "℃".into();
            c
        };
        assert_eq!(ch.raw_to_value(&[0xFE]), -2.0); // -2 as i8
    }

    #[test]
    fn render_shows_the_unit_after_a_physical_value() {
        let mut ch = named(1, 4, DataType::SI, "lat");
        ch.lsb = 4.19e-08;
        ch.unit = "deg".into();

        let rendered = ch.render(1_000_000);

        assert!(rendered.starts_with("0.0419"), "got {rendered}");
        assert!(rendered.ends_with(" deg"), "got {rendered}");
    }

    #[test]
    fn bound_raw_resolves_with_the_current_lsb_and_offset() {
        let mut ch = ChannelDef::new(1, 2, DataType::UI);
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
        let mut ch = ChannelDef::new(1, 1, DataType::SI);
        ch.min = Some(Value::Raw(0xFF));

        assert_eq!(ch.min_value(), Some(-1.0));
    }

    #[test]
    fn range_contains_is_true_when_unbounded_and_never_for_nan() {
        let ch = ChannelDef::new(1, 2, DataType::UI);

        assert!(ch.range_contains(1e9));
        assert!(!ch.range_contains(f64::NAN));
    }

    #[test]
    fn range_queries_use_the_bounds() {
        let mut ch = ChannelDef::new(1, 2, DataType::UI);
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
        let ch = ChannelDef::new(1, 2, DataType::UI);

        assert_eq!((ch.number, ch.byte_count), (1, 2));
        assert_eq!(ch.data_type, DataType::UI);
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
        let c = ch(1, DataType::UI, 1.0, 0.0);
        assert_eq!(c.raw_to_bytes_endian(0x1FF, Endian::Little), vec![0xFF]);
        assert_eq!(c.raw_to_bytes_endian(256, Endian::Little), vec![0x00]);

        let c = ch(2, DataType::UI, 1.0, 0.0);
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
        let c = ch(2, DataType::UI, 1.0, 0.0);
        assert_eq!(
            c.raw_from_bytes_endian(&[0x34, 0x12], Endian::Little),
            0x1234
        );
        assert_eq!(c.raw_from_bytes_endian(&[0x12, 0x34], Endian::Big), 0x1234);

        let c = ch(4, DataType::UI, 1.0, 0.0);
        let bytes = c.raw_to_bytes_endian(0xDEADBEEF, Endian::Big);
        assert_eq!(c.raw_from_bytes_endian(&bytes, Endian::Big), 0xDEADBEEF);
    }

    #[test]
    fn build_layout_skips_bf_rows_whose_parent_is_not_a_bitfield() {
        let channels = vec![ChannelDef::new(1, 2, DataType::UI)];
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
    fn limits_exceeded_reports_only_when_the_frame_does_not_fit() {
        let layout = build_layout(vec![ch(4, DataType::UI, 1.0, 0.0)], vec![]).value;

        assert!(layout.clone().with_capacity(4).limits_exceeded().is_empty());
        let issues = layout.with_capacity(3).limits_exceeded();
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(
            issues[0].code,
            crate::issue::IssueCode::LayoutExceedsCapacity
        );
        assert_eq!(issues[0].row, None);
    }

    #[test]
    fn decode_slices_the_frame_in_row_order_and_omits_overruns() {
        let mut a = ChannelDef::new(1, 2, DataType::UI);
        a.lsb = 0.5;
        let b = ChannelDef::new(2, 1, DataType::UI);
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
        let mut layout = build_layout(vec![ChannelDef::new(1, 2, DataType::UI)], vec![]).value;
        layout.endian = Endian::Big;

        let decoded = layout.decode(&[0x12, 0x34]);
        assert_eq!(decoded[0].raw, 0x1234);
        assert_eq!(decoded[0].value, 0x1234 as f64);
    }

    #[test]
    fn channel_bytes_finds_the_slice_of_one_channel() {
        let layout = build_layout(
            vec![
                ChannelDef::new(1, 2, DataType::UI),
                ChannelDef::new(2, 1, DataType::UI),
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
        let c = ChannelDef::new(1, 2, DataType::UI);

        assert!(c.section.is_empty() && c.memo.is_empty() && c.var.is_empty());
        assert_eq!(c.format, ValueDisplay::Physical);
        assert!(!c.favorite);
    }

    #[test]
    fn display_format_parses_case_insensitively() {
        assert_eq!(ValueDisplay::parse("HEX"), Some(ValueDisplay::Raw));
        assert_eq!(ValueDisplay::parse("dec"), Some(ValueDisplay::Physical));
        assert_eq!(ValueDisplay::parse("octal"), None);
    }

    #[test]
    fn raw_to_value_u64_matches_the_byte_form() {
        let mut c = ch(2, DataType::SI, 0.1, -5.0);
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
        assert_eq!(ch(1, DataType::SI, 1.0, 0.0).raw_to_value_u64(0xFF), -1.0);
        assert_eq!(ch(1, DataType::UI, 1.0, 0.0).raw_to_value_u64(0xFF), 255.0);
        assert_eq!(
            ch(2, DataType::BF, 1.0, 0.0).raw_to_value_u64(0xFFFF),
            65535.0
        );
    }

    #[test]
    fn positions_pair_each_channel_with_its_offset() {
        let layout = build_layout(
            vec![
                ChannelDef::new(1, 4, DataType::UI),
                ChannelDef::new(2, 2, DataType::UI),
                ChannelDef::new(3, 1, DataType::UI),
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
        let mut a = ChannelDef::new(1, 1, DataType::UI);
        a.default_value = Some(0x7E);
        let b = ChannelDef::new(2, 2, DataType::UI);
        let layout = build_layout(vec![a, b], vec![]).value;

        let out = layout.encode(&[]);

        assert!(out.issues.is_empty());
        assert_eq!(out.value, vec![0x7E, 0x00, 0x00]);
    }

    #[test]
    fn encode_converts_physical_and_truncates_raw_values() {
        let mut a = ChannelDef::new(1, 2, DataType::UI);
        a.lsb = 0.5;
        a.offset = -5.0;
        let b = ChannelDef::new(2, 1, DataType::UI);
        let layout = build_layout(vec![a, b], vec![]).value;

        let out = layout.encode(&[(1, Value::Physical(5.0)), (2, Value::Raw(0x1FF))]);

        assert_eq!(out.value, vec![20, 0, 0xFF]);
        // The raw value did not fit channel 2; conversion.md §3 keeps the low
        // bits and reports it rather than dropping the difference in silence.
        assert_eq!(
            out.issues.iter().map(|i| i.code).collect::<Vec<_>>(),
            vec![IssueCode::RawOutOfRange]
        );
    }

    #[test]
    fn encode_respects_the_layout_endian() {
        let mut layout = build_layout(vec![ChannelDef::new(1, 2, DataType::UI)], vec![]).value;
        layout.endian = Endian::Big;

        assert_eq!(
            layout.encode(&[(1, Value::Raw(0x1234))]).value,
            vec![0x12, 0x34]
        );
    }

    #[test]
    fn encode_reports_values_it_cannot_place() {
        let layout = build_layout(vec![ChannelDef::new(1, 2, DataType::UI)], vec![]).value;

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
        let layout = build_layout(vec![ChannelDef::new(1, 1, DataType::UI)], vec![]).value;

        let out = layout.encode(&[(1, Value::Raw(1)), (1, Value::Raw(2))]);

        assert_eq!(out.value, vec![2]);
    }

    #[test]
    fn decode_reads_back_what_encode_wrote() {
        let mut a = ChannelDef::new(1, 2, DataType::SI);
        a.lsb = 0.1;
        let b = ChannelDef::new(2, 4, DataType::UI);
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
            |number: u32, byte_count: usize| ChannelDef::new(number, byte_count, DataType::UI);

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
