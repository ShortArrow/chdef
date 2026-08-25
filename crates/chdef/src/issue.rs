//! Per-row problems found while loading (`docs/spec/diagnostics.md`).
//!
//! A problem in one row never stops loading: every readable row is read and
//! the problem comes back as an [`Issue`] next to the value ([`Parsed`]).
//! Loading fails outright only with a [`crate::ChdefError`] — an I/O
//! failure, a structurally broken CSV, or bytes that are not UTF-8.

/// A per-row problem found while loading; loading continued.
///
/// Everything a consumer needs to write its own sentence is a field: the
/// stable [`code`](Issue::code), where in the file the finding is, which
/// channel or bit it is about, the value that could not be used and the
/// value used instead. [`message`](Issue::message) is an English rendering
/// of the same facts, for a log — its wording is not part of the contract
/// and may change in any release.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub code: IssueCode,
    /// 0-based data row (header excluded, skipped rows counted), so it maps
    /// directly onto a grid row. `None` when not tied to a row.
    pub row: Option<usize>,
    /// 0-based column position in the file. `None` when not tied to a column.
    pub col: Option<usize>,
    /// The channel the finding is about, when it is about one. The only way
    /// to name the row for a finding that carries none.
    pub channel: Option<u32>,
    /// The bit, for a finding about one bit of a `BF` channel.
    pub bit: Option<u8>,
    /// The value chdef could not use, spelled as the file spells it — the
    /// one thing parsing throws away.
    pub found: Option<String>,
    /// The value chdef used instead, where it substituted one.
    pub used: Option<String>,
    /// One English sentence saying the same as the fields above.
    pub message: String,
}

impl Issue {
    pub(crate) fn new(code: IssueCode, message: String) -> Issue {
        Issue {
            code,
            row: None,
            col: None,
            channel: None,
            bit: None,
            found: None,
            used: None,
            message,
        }
    }

    pub(crate) fn at(mut self, row: usize, col: Option<usize>) -> Issue {
        self.row = Some(row);
        self.col = col;
        self
    }

    pub(crate) fn about_channel(mut self, channel: u32) -> Issue {
        self.channel = Some(channel);
        self
    }

    pub(crate) fn about_bit(mut self, channel: u32, bit: u8) -> Issue {
        self.channel = Some(channel);
        self.bit = Some(bit);
        self
    }

    pub(crate) fn found(mut self, found: impl Into<String>) -> Issue {
        self.found = Some(found.into());
        self
    }

    pub(crate) fn used(mut self, used: impl Into<String>) -> Issue {
        self.used = Some(used.into());
        self
    }
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
    /// The channel shows its raw value (`format` is `HEX`) while `lsb` is
    /// not 1, so the number shown is not the physical quantity.
    RawDisplayWithLsb,
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
    /// `total_bytes` exceeds the capacity handed to `limits_exceeded`.
    LayoutExceedsCapacity,
    /// The layout has more channels than the port stated it accepts.
    LayoutExceedsChannelCapacity,
    /// `kind` is not a kind this chdef knows; `plain` was assumed.
    KindAssumed,
    /// An encode value names a channel the layout does not have. Ignored.
    EncodeUnknownChannel,
    /// An encode value cannot be converted (NaN / infinite). The channel's
    /// default was used.
    EncodeValueInvalid,
    /// An encode value does not fit the channel width. The clamped value
    /// was written.
    EncodeValueClamped,
    /// A value lies outside its channel's declared range. Nothing was
    /// changed; `used` is the bound it crossed.
    ValueOutOfRange,
    /// The `derived` cell is not a recipe this chdef knows. The channel
    /// keeps its `default`.
    DerivedInvalid,
    /// A recipe covers a channel the layout does not have, or the frame is
    /// too short for one it covers. Nothing was computed.
    DerivedUnknownChannel,
    /// A derived channel disagrees with its recipe. `found` is the stored
    /// value, `used` the computed one.
    DerivedMismatch,
    /// The recipe reads, and names an algorithm this chdef does not
    /// compute. Its coverage is still available through `covered_bytes`.
    DerivedUnknownRecipe,
}

const ALL_ISSUE_CODES: [IssueCode; 32] = [
    IssueCode::HeaderAssumed,
    IssueCode::ChannelNumberInvalid,
    IssueCode::ChannelDuplicate,
    IssueCode::BytesAssumed,
    IssueCode::BytesOutOfRange,
    IssueCode::TypeAssumed,
    IssueCode::TypeWidthMismatch,
    IssueCode::LsbInvalid,
    IssueCode::OffsetInvalid,
    IssueCode::DefaultInvalid,
    IssueCode::RawDisplayWithLsb,
    IssueCode::RawOutOfRange,
    IssueCode::MinInvalid,
    IssueCode::MaxInvalid,
    IssueCode::MinMaxSwapped,
    IssueCode::BfParentInvalid,
    IssueCode::BfBitInvalid,
    IssueCode::BfDefaultInvalid,
    IssueCode::BfDuplicate,
    IssueCode::BfBitOutOfRange,
    IssueCode::BfParentNotBitfield,
    IssueCode::LayoutExceedsCapacity,
    IssueCode::LayoutExceedsChannelCapacity,
    IssueCode::KindAssumed,
    IssueCode::EncodeUnknownChannel,
    IssueCode::EncodeValueInvalid,
    IssueCode::EncodeValueClamped,
    IssueCode::ValueOutOfRange,
    IssueCode::DerivedInvalid,
    IssueCode::DerivedUnknownChannel,
    IssueCode::DerivedMismatch,
    IssueCode::DerivedUnknownRecipe,
];

impl IssueCode {
    /// Every code this chdef can report, in declaration order.
    ///
    /// The codes are carried as strings so the vocabulary can grow
    /// (ADR-0021); this is what lets a consumer prove its own table
    /// covers them (ADR-0026) instead of finding a gap when a count moves.
    pub fn all() -> &'static [IssueCode] {
        &ALL_ISSUE_CODES
    }

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
            IssueCode::RawDisplayWithLsb => "raw_display_with_lsb",
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
            IssueCode::LayoutExceedsChannelCapacity => "layout_exceeds_channel_capacity",
            IssueCode::KindAssumed => "kind_assumed",
            IssueCode::EncodeUnknownChannel => "encode_unknown_channel",
            IssueCode::EncodeValueInvalid => "encode_value_invalid",
            IssueCode::EncodeValueClamped => "encode_value_clamped",
            IssueCode::ValueOutOfRange => "value_out_of_range",
            IssueCode::DerivedInvalid => "derived_invalid",
            IssueCode::DerivedUnknownChannel => "derived_unknown_channel",
            IssueCode::DerivedMismatch => "derived_mismatch",
            IssueCode::DerivedUnknownRecipe => "derived_unknown_recipe",
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
