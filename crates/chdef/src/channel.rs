use serde::{Deserialize, Serialize};

/// Byte order of multi-byte channels on the wire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Endian {
    #[default]
    Little,
    Big,
}

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

    pub fn resolve(parsed: Self, byte_count: usize) -> Self {
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

    pub fn category(&self) -> &'static str {
        match self {
            DataType::UI8 | DataType::UI16 | DataType::UI32 => "UI",
            DataType::SI8 | DataType::SI16 | DataType::SI32 => "SI",
            DataType::BF => "BF",
        }
    }

    pub fn is_bitfield(&self) -> bool {
        matches!(self, DataType::BF)
    }

    pub fn parse(s: &str) -> Option<(Self, usize)> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        let prefix = if s.len() >= 2 { &s[..2] } else { s };
        let bits_str = if s.len() > 2 { &s[2..] } else { "" };
        let bits: Option<usize> = bits_str.parse().ok();

        match (prefix.to_uppercase().as_str(), bits) {
            ("UI", Some(8)) => Some((DataType::UI8, 1)),
            ("UI", Some(16)) | ("UI", None) => Some((DataType::UI16, 2)),
            ("UI", Some(32)) => Some((DataType::UI32, 4)),
            ("SI", Some(8)) => Some((DataType::SI8, 1)),
            ("SI", Some(16)) | ("SI", None) => Some((DataType::SI16, 2)),
            ("SI", Some(32)) => Some((DataType::SI32, 4)),
            ("BF", _) => Some((DataType::BF, 2)),
            _ => None,
        }
    }
}

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
}

impl ChannelDef {
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
        let raw = self.value_to_raw(value)?;
        let mut bytes = raw.to_le_bytes()[..self.byte_count.min(8)].to_vec();
        if endian == Endian::Big {
            bytes.reverse();
        }
        Some(bytes)
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

#[derive(Debug, Clone)]
pub struct BitFieldDef {
    pub parent_channel: u32,
    pub bit_number: u8,
    pub name: String,
    /// Optional protocol-spec default of the bit (`0` / `1`); `None` keeps
    /// the default bit of the parent channel.
    pub default_value: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct ChannelLayout {
    pub channels: Vec<ChannelDef>,
    pub bitfields: Vec<BitFieldDef>,
    pub total_bytes: usize,
}

/// Build the Layout stage from parsed Rows: duplicates are dropped (the
/// first `number`, and the first `(number, bit)`, win; the parser already
/// reported them as Issues) and `total_bytes` is the sum of the remaining
/// channel widths in row order.
pub fn build_layout(channels: Vec<ChannelDef>, bitfields: Vec<BitFieldDef>) -> ChannelLayout {
    let mut unique_channels: Vec<ChannelDef> = Vec::new();
    for ch in channels {
        if !unique_channels.iter().any(|c| c.number == ch.number) {
            unique_channels.push(ch);
        }
    }
    let mut unique_bitfields: Vec<BitFieldDef> = Vec::new();
    for bf in bitfields {
        if !unique_bitfields
            .iter()
            .any(|b| b.parent_channel == bf.parent_channel && b.bit_number == bf.bit_number)
        {
            unique_bitfields.push(bf);
        }
    }
    let total_bytes = unique_channels.iter().map(|ch| ch.byte_count).sum();
    ChannelLayout {
        channels: unique_channels,
        bitfields: unique_bitfields,
        total_bytes,
    }
}

impl ChannelLayout {
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
        ChannelDef {
            number: 1,
            name: "t".into(),
            byte_count,
            data_type,
            lsb,
            offset,
            unit: String::new(),
            default_value: None,
        }
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
    fn parse_data_type_ui32() {
        let (dt, bytes) = DataType::parse("UI32").unwrap();
        assert_eq!(dt, DataType::UI32);
        assert_eq!(bytes, 4);
    }

    #[test]
    fn parse_data_type_si16() {
        let (dt, bytes) = DataType::parse("SI16").unwrap();
        assert_eq!(dt, DataType::SI16);
        assert_eq!(bytes, 2);
    }

    #[test]
    fn parse_data_type_bf() {
        let (dt, bytes) = DataType::parse("BF").unwrap();
        assert_eq!(dt, DataType::BF);
        assert_eq!(bytes, 2);
    }

    #[test]
    fn parse_data_type_ui_no_bits_defaults_16() {
        let (dt, bytes) = DataType::parse("UI").unwrap();
        assert_eq!(dt, DataType::UI16);
        assert_eq!(bytes, 2);
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(DataType::parse("").is_none());
    }

    #[test]
    fn build_layout_total_bytes() {
        let channels = vec![
            ChannelDef {
                number: 1,
                name: "a".into(),
                byte_count: 4,
                data_type: DataType::UI32,
                lsb: 1.0,
                offset: 0.0,
                unit: String::new(),
                default_value: None,
            },
            ChannelDef {
                number: 2,
                name: "b".into(),
                byte_count: 2,
                data_type: DataType::UI16,
                lsb: 1.0,
                offset: 0.0,
                unit: String::new(),
                default_value: None,
            },
            ChannelDef {
                number: 3,
                name: "c".into(),
                byte_count: 1,
                data_type: DataType::UI8,
                lsb: 1.0,
                offset: 0.0,
                unit: String::new(),
                default_value: None,
            },
        ];
        let layout = build_layout(channels, vec![]);
        assert_eq!(layout.total_bytes, 7);
        assert_eq!(layout.channels.len(), 3);
    }

    #[test]
    fn raw_to_value_identity() {
        let ch = ChannelDef {
            number: 1,
            name: "test".into(),
            byte_count: 2,
            data_type: DataType::UI16,
            lsb: 1.0,
            offset: 0.0,
            unit: String::new(),
            default_value: None,
        };
        assert_eq!(ch.raw_to_value(&[0x2A, 0x00]), 42.0);
    }

    #[test]
    fn raw_to_value_with_lsb_and_offset() {
        let ch = ChannelDef {
            number: 1,
            name: "lat".into(),
            byte_count: 4,
            data_type: DataType::SI32,
            lsb: 4.19e-08,
            offset: 0.0,
            unit: "deg".into(),
            default_value: None,
        };
        let raw = 1_000_000i32.to_le_bytes();
        let val = ch.raw_to_value(&raw);
        assert!((val - 0.0419).abs() < 0.001);
    }

    #[test]
    fn raw_to_value_lsb_zero_treated_as_identity() {
        let ch = ChannelDef {
            number: 1,
            name: "test".into(),
            byte_count: 2,
            data_type: DataType::UI16,
            lsb: 0.0,
            offset: 0.0,
            unit: String::new(),
            default_value: None,
        };
        assert_eq!(ch.raw_to_value(&[0x0A, 0x00]), 10.0);
    }

    #[test]
    fn raw_to_value_signed() {
        let ch = ChannelDef {
            number: 1,
            name: "temp".into(),
            byte_count: 1,
            data_type: DataType::SI8,
            lsb: 1.0,
            offset: 0.0,
            unit: "℃".into(),
            default_value: None,
        };
        assert_eq!(ch.raw_to_value(&[0xFE]), -2.0); // -2 as i8
    }

    #[test]
    fn format_value_integer_when_lsb_is_one() {
        let ch = ChannelDef {
            number: 1,
            name: "cnt".into(),
            byte_count: 2,
            data_type: DataType::UI16,
            lsb: 1.0,
            offset: 0.0,
            unit: String::new(),
            default_value: None,
        };
        assert_eq!(ch.format_value(&[0x2A, 0x00]), "42");
    }

    #[test]
    fn format_value_decimal_when_lsb_is_fractional() {
        let ch = ChannelDef {
            number: 1,
            name: "lat".into(),
            byte_count: 4,
            data_type: DataType::SI32,
            lsb: 4.19e-08,
            offset: 0.0,
            unit: "deg".into(),
            default_value: None,
        };
        let raw = 1_000_000i32.to_le_bytes();
        let formatted = ch.format_value(&raw);
        assert!(formatted.contains("0.0419"), "Got: {}", formatted);
    }

    #[test]
    fn build_layout_keeps_the_first_of_duplicate_numbers() {
        let dup = |number: u32, byte_count: usize| ChannelDef {
            number,
            name: String::new(),
            byte_count,
            data_type: DataType::UI16,
            lsb: 1.0,
            offset: 0.0,
            unit: String::new(),
            default_value: None,
        };

        let layout = build_layout(vec![dup(1, 2), dup(1, 4), dup(2, 1)], vec![]);

        assert_eq!(layout.channels.len(), 2);
        assert_eq!(layout.channels[0].byte_count, 2);
        assert_eq!(layout.total_bytes, 3);
    }

    #[test]
    fn build_layout_keeps_the_first_of_duplicate_bits() {
        let bf = |bit: u8, name: &str| BitFieldDef {
            parent_channel: 2,
            bit_number: bit,
            name: name.into(),
            default_value: None,
        };

        let layout = build_layout(
            vec![],
            vec![bf(0, "first"), bf(0, "second"), bf(1, "other")],
        );

        assert_eq!(layout.bitfields.len(), 2);
        assert_eq!(layout.bitfields[0].name, "first");
    }
}
