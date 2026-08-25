// Every Issue code, as constants and as a list.
//
// The codes cross the ABI as strings so the vocabulary can grow
// (ADR-0021), which left a caller holding a string with nothing to check
// it against: a misspelling compiled and fell through to the English
// Message, the one field whose wording is not contracted. These constants
// catch that, and All lets a table keyed by code be proved complete
// (ADR-0026).
//
// They are not an enum on purpose. A code this assembly has not heard of
// still arrives as the string it is, rather than having nowhere to land.

namespace Chdef;

/// <summary>
/// The stable identifiers <see cref="Issue.Code"/> carries. A test asserts
/// these are exactly what the native library enumerates, in order, so this
/// file is a checked mirror and never a second list.
/// </summary>
public static class IssueCode
{
    /// <summary>No header, or no <c>number</c> column; the positional order was assumed.</summary>
    public const string HeaderAssumed = "header_assumed";

    /// <summary><c>number</c> is not an integer ≥ 1. Row skipped.</summary>
    public const string ChannelNumberInvalid = "channel_number_invalid";

    /// <summary>The same <c>number</c> already exists. The layout keeps the first row.</summary>
    public const string ChannelDuplicate = "channel_duplicate";

    /// <summary><c>bytes</c> empty / non-integer. The width of <c>type</c> (or 2) was assumed.</summary>
    public const string BytesAssumed = "bytes_assumed";

    /// <summary><c>bytes</c> outside 1–8. Clamped.</summary>
    public const string BytesOutOfRange = "bytes_out_of_range";

    /// <summary><c>type</c> empty / unknown. <c>UI</c> was assumed.</summary>
    public const string TypeAssumed = "type_assumed";

    /// <summary>The width suffix of <c>type</c> disagrees with <c>bytes</c>. <c>bytes</c> wins.</summary>
    public const string TypeWidthMismatch = "type_width_mismatch";

    /// <summary><c>lsb</c> is not a finite number. 1 was used.</summary>
    public const string LsbInvalid = "lsb_invalid";

    /// <summary><c>offset</c> is not a number. 0 was used.</summary>
    public const string OffsetInvalid = "offset_invalid";

    /// <summary><c>default</c> is neither an integer nor <c>0x</c>. Treated as unspecified.</summary>
    public const string DefaultInvalid = "default_invalid";

    /// <summary>The channel shows its raw value (<c>format</c> is <c>HEX</c>) while <c>lsb</c> is not 1, so the number shown is not the physical quantity.</summary>
    public const string RawDisplayWithLsb = "raw_display_with_lsb";

    /// <summary>A <c>0x</c> raw value exceeds the width. The low bits were used.</summary>
    public const string RawOutOfRange = "raw_out_of_range";

    /// <summary><c>min</c> is neither a number nor <c>0x</c>. Treated as unspecified.</summary>
    public const string MinInvalid = "min_invalid";

    /// <summary><c>max</c> is neither a number nor <c>0x</c>. Treated as unspecified.</summary>
    public const string MaxInvalid = "max_invalid";

    /// <summary>The resolved <c>min</c> exceeds the resolved <c>max</c>. Both kept; the range matches nothing.</summary>
    public const string MinMaxSwapped = "min_max_swapped";

    /// <summary>BF <c>number</c> is not an integer. Row skipped.</summary>
    public const string BfParentInvalid = "bf_parent_invalid";

    /// <summary>BF <c>bit</c> is not an integer. Row skipped.</summary>
    public const string BfBitInvalid = "bf_bit_invalid";

    /// <summary>BF <c>default</c> is not <c>0</c> / <c>1</c>. Treated as unspecified.</summary>
    public const string BfDefaultInvalid = "bf_default_invalid";

    /// <summary>The same <c>(number, bit)</c> already exists. The layout keeps the first row.</summary>
    public const string BfDuplicate = "bf_duplicate";

    /// <summary>BF <c>bit</c> is at or beyond the parent width. Row skipped by the layout.</summary>
    public const string BfBitOutOfRange = "bf_bit_out_of_range";

    /// <summary>BF parent channel missing, or its <c>type</c> is not <c>BF</c>. Row skipped by the layout.</summary>
    public const string BfParentNotBitfield = "bf_parent_not_bitfield";

    /// <summary><c>total_bytes</c> exceeds the capacity handed to <c>limits_exceeded</c>.</summary>
    public const string LayoutExceedsCapacity = "layout_exceeds_capacity";

    /// <summary>The layout has more channels than the port stated it accepts.</summary>
    public const string LayoutExceedsChannelCapacity = "layout_exceeds_channel_capacity";

    /// <summary><c>kind</c> is not a kind this chdef knows; <c>plain</c> was assumed.</summary>
    public const string KindAssumed = "kind_assumed";

    /// <summary>An encode value names a channel the layout does not have. Ignored.</summary>
    public const string EncodeUnknownChannel = "encode_unknown_channel";

    /// <summary>An encode value cannot be converted (NaN / infinite). The channel's default was used.</summary>
    public const string EncodeValueInvalid = "encode_value_invalid";

    /// <summary>An encode value does not fit the channel width. The clamped value was written.</summary>
    public const string EncodeValueClamped = "encode_value_clamped";

    /// <summary>A value lies outside its channel's declared range. Nothing was changed; <c>used</c> is the bound it crossed.</summary>
    public const string ValueOutOfRange = "value_out_of_range";

    /// <summary>
    /// Every code the loaded native library can report, in its order. Read
    /// from the library rather than from this file, so a code added there
    /// appears here without a rebuild.
    /// </summary>
    public static IReadOnlyList<string> All => Enumerate();

    private static unsafe List<string> Enumerate()
    {
        var count = (int)Native.chdef_issue_code_count();
        var codes = new List<string>(count);
        for (var i = 0; i < count; i++)
        {
            var index = (nuint)i;
            codes.Add(TextBuffer.Read((buf, cap) => Native.chdef_issue_code_name(index, buf, cap)));
        }
        return codes;
    }
}
