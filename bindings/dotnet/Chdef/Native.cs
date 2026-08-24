// The P/Invoke declarations of crates/chdef-capi/include/chdef.h.
//
// This file is a mirror of that header and of nothing else. A test in the
// Rust workspace (crates/chdef-capi/tests/dotnet_binding.rs) asserts that
// every exported function, constant and struct field appears here, in
// order and with the mapped type — a mis-declared field is silent memory
// corruption, which no golden vector can catch (ADR-0022).

using System.Runtime.InteropServices;

namespace Chdef;

/// <summary>One channel of the layout, as the ABI describes it.</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct ChdefChannel
{
    public uint Number;
    public ulong At;
    public ulong Bytes;
    public double Lsb;
    public double Offset;
    /// <summary>-1 when the channel states no default.</summary>
    public long DefaultValue;
    public int Favorite;
    public ulong BitCount;
}

/// <summary>One diagnostic. Fields are -1 when the finding carries none.</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct ChdefIssue
{
    public long Row;
    public long Col;
    public long Channel;
    public long Bit;
}

/// <summary>A value handed to encode. Kind 0 takes Physical, 1 takes Raw.</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct ChdefValue
{
    public uint Channel;
    public int Kind;
    public double Physical;
    public ulong Raw;
}

/// <summary>One channel's readings from a decoded frame.</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct ChdefReading
{
    public uint Channel;
    public ulong Raw;
    public double Value;
}

/// <summary>One named bit of a channel. DefaultValue is 0 or 1, or -1 when
/// the BF row names none and the bit keeps the parent channel's.</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct ChdefBit
{
    public uint Channel;
    public uint Bit;
    public int DefaultValue;
}

/// <summary>One named bit of a decoded frame, and whether it is set.</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct ChdefBitReading
{
    public uint Channel;
    public uint Bit;
    public int Value;
}

internal static unsafe partial class Native
{
    internal const string Library = "chdef_capi";

    internal const uint CHDEF_ABI_VERSION = 3u;

    internal const int CHDEF_OK = 0;
    internal const int CHDEF_ERR_HANDLE = -1;
    internal const int CHDEF_ERR_INDEX = -2;
    internal const int CHDEF_ERR_BUFFER = -3;
    internal const int CHDEF_ERR_NULL = -4;
    internal const int CHDEF_ERR_UTF8 = -5;
    internal const int CHDEF_ERR_CSV = -6;
    internal const int CHDEF_ERR_IO = -7;
    internal const int CHDEF_ERR_VALUE = -8;
    internal const int CHDEF_ERR_COLUMN = -9;
    internal const int CHDEF_PANIC = -99;

    internal const int CHDEF_LITTLE = 0;
    internal const int CHDEF_BIG = 1;

    internal const int CHDEF_CHANNEL_NAME = 0;
    internal const int CHDEF_CHANNEL_TYPE = 1;
    internal const int CHDEF_CHANNEL_UNIT = 2;
    internal const int CHDEF_CHANNEL_SECTION = 3;
    internal const int CHDEF_CHANNEL_MEMO = 4;
    internal const int CHDEF_CHANNEL_VAR = 5;
    internal const int CHDEF_CHANNEL_FORMAT = 6;
    internal const int CHDEF_CHANNEL_MIN = 7;
    internal const int CHDEF_CHANNEL_MAX = 8;

    internal const int CHDEF_ISSUE_CODE = 0;
    internal const int CHDEF_ISSUE_FOUND = 1;
    internal const int CHDEF_ISSUE_USED = 2;
    internal const int CHDEF_ISSUE_MESSAGE = 3;

    internal const int CHDEF_BIT_NAME = 0;
    internal const int CHDEF_BIT_MEMO = 1;

    internal const int CHDEF_COLUMNS_CH = 0;
    internal const int CHDEF_COLUMNS_BF = 1;

    [LibraryImport(Library)]
    internal static partial uint chdef_abi_version();

    [LibraryImport(Library)]
    internal static partial int chdef_layout_parse(
        byte* ch, nuint chLen,
        byte* bf, nuint bfLen,
        nint* outLayout,
        nint* outIssues,
        byte* errBuf, nuint errCap);

    [LibraryImport(Library)]
    internal static partial void chdef_layout_free(nint handle);

    [LibraryImport(Library)]
    internal static partial void chdef_issues_free(nint handle);

    [LibraryImport(Library)]
    internal static partial ulong chdef_layout_total_bytes(nint handle);

    [LibraryImport(Library)]
    internal static partial ulong chdef_layout_channel_count(nint handle);

    [LibraryImport(Library)]
    internal static partial int chdef_layout_channel_at(
        nint handle, nuint index, ChdefChannel* outChannel);

    [LibraryImport(Library)]
    internal static partial nuint chdef_layout_channel_text(
        nint handle, nuint index, int field, byte* buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial int chdef_layout_set_endian(nint handle, int endian);

    [LibraryImport(Library)]
    internal static partial int chdef_layout_set_capacity(nint handle, ulong capacity);

    [LibraryImport(Library)]
    internal static partial int chdef_layout_check_capacity(
        nint handle, nint* outIssues);

    [LibraryImport(Library)]
    internal static partial int chdef_encode(
        nint handle, ChdefValue* values, nuint valueCount,
        byte* frame, nuint frameCap, nuint* outLen, nint* outIssues);

    [LibraryImport(Library)]
    internal static partial int chdef_decode(
        nint handle, byte* frame, nuint frameLen,
        ChdefReading* outReadings, nuint outCap, nuint* outCount);

    [LibraryImport(Library)]
    internal static partial ulong chdef_column_count(int kind);

    [LibraryImport(Library)]
    internal static partial nuint chdef_column_name(
        int kind, nuint index, byte* buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial int chdef_vocabulary_new(nint* outVocabulary);

    [LibraryImport(Library)]
    internal static partial int chdef_vocabulary_japanese(nint* outVocabulary);

    [LibraryImport(Library)]
    internal static partial void chdef_vocabulary_free(nint handle);

    [LibraryImport(Library)]
    internal static partial int chdef_vocabulary_teach(
        nint handle, int kind,
        byte* spelling, nuint spellingLen,
        byte* column, nuint columnLen);

    [LibraryImport(Library)]
    internal static partial int chdef_layout_parse_with(
        byte* ch, nuint chLen,
        byte* bf, nuint bfLen,
        nint vocabulary,
        nint* outLayout,
        nint* outIssues,
        byte* errBuf, nuint errCap);

    [LibraryImport(Library)]
    internal static partial int chdef_layout_bit_at(
        nint handle, nuint channelIndex, nuint bitIndex, ChdefBit* outBit);

    [LibraryImport(Library)]
    internal static partial nuint chdef_layout_bit_text(
        nint handle, nuint channelIndex, nuint bitIndex, int field,
        byte* buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial ulong chdef_layout_bit_total(nint handle);

    [LibraryImport(Library)]
    internal static partial int chdef_decode_bits(
        nint handle, byte* frame, nuint frameLen,
        ChdefBitReading* outBits, nuint outCap, nuint* outCount);

    [LibraryImport(Library)]
    internal static partial int chdef_layout_channel_displayed(
        nint handle, nuint index, ulong raw, ChdefValue* outValue);

    [LibraryImport(Library)]
    internal static partial nuint chdef_layout_channel_render(
        nint handle, nuint index, ulong raw, byte* buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial int chdef_value_parse(
        byte* text, nuint len, uint channel, ChdefValue* outValue);

    [LibraryImport(Library)]
    internal static partial int chdef_grid_parse(
        byte* bytes, nuint len, nint* outGrid, byte* errBuf, nuint errCap);

    [LibraryImport(Library)]
    internal static partial void chdef_grid_free(nint handle);

    [LibraryImport(Library)]
    internal static partial ulong chdef_grid_row_count(nint handle);

    [LibraryImport(Library)]
    internal static partial ulong chdef_grid_header_count(nint handle);

    [LibraryImport(Library)]
    internal static partial ulong chdef_grid_col_count(nint handle, nuint row);

    [LibraryImport(Library)]
    internal static partial nuint chdef_grid_header_at(
        nint handle, nuint col, byte* buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial nuint chdef_grid_cell(
        nint handle, nuint row, nuint col, byte* buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial int chdef_grid_set_cell(
        nint handle, nuint row, nuint col, byte* value, nuint len);

    [LibraryImport(Library)]
    internal static partial int chdef_grid_insert_row(nint handle, nuint at);

    [LibraryImport(Library)]
    internal static partial int chdef_grid_append_row(nint handle);

    [LibraryImport(Library)]
    internal static partial int chdef_grid_remove_row(nint handle, nuint at);

    [LibraryImport(Library)]
    internal static partial nuint chdef_grid_to_csv(
        nint handle, byte* buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial ulong chdef_issue_count(nint handle);

    [LibraryImport(Library)]
    internal static partial int chdef_issue_at(
        nint handle, nuint index, ChdefIssue* outIssue);

    [LibraryImport(Library)]
    internal static partial nuint chdef_issue_text(
        nint handle, nuint index, int field, byte* buf, nuint cap);
}
