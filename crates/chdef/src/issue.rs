//! Per-row problems found while loading (`docs/spec/diagnostics.md`).
//!
//! A problem in one row never stops loading: every readable row is read and
//! the problem comes back as an [`Issue`] next to the value ([`Parsed`]).
//! Loading fails outright only with a [`crate::ChdefError`] — an I/O
//! failure, a structurally broken CSV, or bytes that are not UTF-8.

/// A per-row problem found while loading; loading continued.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub code: IssueCode,
    /// 0-based data row (header excluded, skipped rows counted), so it maps
    /// directly onto a grid row. `None` when not tied to a row.
    pub row: Option<usize>,
    /// 0-based column position in the file. `None` when not tied to a column.
    pub col: Option<usize>,
    /// One English sentence: what was found and what chdef did about it.
    pub message: String,
}

/// What an [`Issue`] is about. Consumers key localisation and filtering on
/// the stable ASCII identifier ([`IssueCode::as_str`]); new codes may appear
/// in any release, so matches need a catch-all arm.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueCode {
    /// No header, or no `number` column; the positional order was assumed.
    HeaderAssumed,
    /// `number` is not an integer ≥ 1. Row skipped.
    ChannelNumberInvalid,
    /// The same `number` already exists. The layout keeps the first row.
    ChannelDuplicate,
    /// `bytes` empty / non-integer. The width of `type` (or 2) was assumed.
    BytesAssumed,
    /// `bytes` outside 1–8. Clamped.
    BytesOutOfRange,
    /// `type` empty / unknown. `UI` was assumed.
    TypeAssumed,
    /// The width suffix of `type` disagrees with `bytes`. `bytes` wins.
    TypeWidthMismatch,
    /// `lsb` is not a finite number. 1 was used.
    LsbInvalid,
    /// `offset` is not a number. 0 was used.
    OffsetInvalid,
    /// `default` is neither an integer nor `0x`. Treated as unspecified.
    DefaultInvalid,
    /// `format` is `HEX` but `lsb` is not 1.
    HexWithLsb,
    /// A `0x` raw value exceeds the width. The low bits were used.
    RawOutOfRange,
    /// `min` is neither a number nor `0x`. Treated as unspecified.
    MinInvalid,
    /// `max` is neither a number nor `0x`. Treated as unspecified.
    MaxInvalid,
    /// The resolved `min` exceeds the resolved `max`. Both kept; the range
    /// matches nothing.
    MinMaxSwapped,
    /// BF `number` is not an integer. Row skipped.
    BfParentInvalid,
    /// BF `bit` is not an integer. Row skipped.
    BfBitInvalid,
    /// BF `default` is not `0` / `1`. Treated as unspecified.
    BfDefaultInvalid,
    /// The same `(number, bit)` already exists. The layout keeps the first row.
    BfDuplicate,
    /// BF `bit` is at or beyond the parent width. Row skipped by the layout.
    BfBitOutOfRange,
    /// BF parent channel missing, or its `type` is not `BF`. Row skipped by
    /// the layout.
    BfParentNotBitfield,
    /// `total_bytes` exceeds the capacity handed to `check_capacity`.
    LayoutExceedsCapacity,
    /// An encode value names a channel the layout does not have. Ignored.
    EncodeUnknownChannel,
    /// An encode value cannot be converted (NaN / infinite). The channel's
    /// default was used.
    EncodeValueInvalid,
}

impl IssueCode {
    /// The stable ASCII identifier of the code.
    pub fn as_str(self) -> &'static str {
        match self {
            IssueCode::HeaderAssumed => "header_assumed",
            IssueCode::ChannelNumberInvalid => "channel_number_invalid",
            IssueCode::ChannelDuplicate => "channel_duplicate",
            IssueCode::BytesAssumed => "bytes_assumed",
            IssueCode::BytesOutOfRange => "bytes_out_of_range",
            IssueCode::TypeAssumed => "type_assumed",
            IssueCode::TypeWidthMismatch => "type_width_mismatch",
            IssueCode::LsbInvalid => "lsb_invalid",
            IssueCode::OffsetInvalid => "offset_invalid",
            IssueCode::DefaultInvalid => "default_invalid",
            IssueCode::HexWithLsb => "hex_with_lsb",
            IssueCode::RawOutOfRange => "raw_out_of_range",
            IssueCode::MinInvalid => "min_invalid",
            IssueCode::MaxInvalid => "max_invalid",
            IssueCode::MinMaxSwapped => "min_max_swapped",
            IssueCode::BfParentInvalid => "bf_parent_invalid",
            IssueCode::BfBitInvalid => "bf_bit_invalid",
            IssueCode::BfDefaultInvalid => "bf_default_invalid",
            IssueCode::BfDuplicate => "bf_duplicate",
            IssueCode::BfBitOutOfRange => "bf_bit_out_of_range",
            IssueCode::BfParentNotBitfield => "bf_parent_not_bitfield",
            IssueCode::LayoutExceedsCapacity => "layout_exceeds_capacity",
            IssueCode::EncodeUnknownChannel => "encode_unknown_channel",
            IssueCode::EncodeValueInvalid => "encode_value_invalid",
        }
    }
}

impl std::fmt::Display for IssueCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A parsed value plus the [`Issue`]s found on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed<T> {
    pub value: T,
    pub issues: Vec<Issue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_codes_spell_their_stable_identifier() {
        assert_eq!(IssueCode::HeaderAssumed.as_str(), "header_assumed");
        assert_eq!(IssueCode::BfDuplicate.to_string(), "bf_duplicate");
        assert_eq!(IssueCode::MinInvalid.as_str(), "min_invalid");
        assert_eq!(IssueCode::MaxInvalid.as_str(), "max_invalid");
        assert_eq!(IssueCode::MinMaxSwapped.as_str(), "min_max_swapped");
        assert_eq!(IssueCode::BfBitOutOfRange.as_str(), "bf_bit_out_of_range");
        assert_eq!(
            IssueCode::BfParentNotBitfield.as_str(),
            "bf_parent_not_bitfield"
        );
        assert_eq!(
            IssueCode::LayoutExceedsCapacity.as_str(),
            "layout_exceeds_capacity"
        );
        assert_eq!(
            IssueCode::EncodeUnknownChannel.as_str(),
            "encode_unknown_channel"
        );
        assert_eq!(
            IssueCode::EncodeValueInvalid.as_str(),
            "encode_value_invalid"
        );
    }
}
