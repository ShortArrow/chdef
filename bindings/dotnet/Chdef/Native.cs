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

internal static unsafe partial class Native
{
    internal const string Library = "chdef_capi";

    internal const uint CHDEF_ABI_VERSION = 1u;

    internal const int CHDEF_OK = 0;
    internal const int CHDEF_ERR_HANDLE = -1;
    internal const int CHDEF_ERR_INDEX = -2;
    internal const int CHDEF_ERR_BUFFER = -3;
    internal const int CHDEF_ERR_NULL = -4;
    internal const int CHDEF_ERR_UTF8 = -5;
    internal const int CHDEF_ERR_CSV = -6;
    internal const int CHDEF_ERR_IO = -7;
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
    internal static partial ulong chdef_issue_count(nint handle);

    [LibraryImport(Library)]
    internal static partial int chdef_issue_at(
        nint handle, nuint index, ChdefIssue* outIssue);

    [LibraryImport(Library)]
    internal static partial nuint chdef_issue_text(
        nint handle, nuint index, int field, byte* buf, nuint cap);
}
