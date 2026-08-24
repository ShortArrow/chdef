// The safe surface over Native.cs: handles that dispose themselves, the
// two-call buffer dance for every string done once, and a status turned
// into an exception. Nothing here reaches past the ABI.

using System.Text;

namespace Chdef;

/// <summary>Byte order of every multi-byte channel of the frame.</summary>
public enum Endian
{
    Little = Native.CHDEF_LITTLE,
    Big = Native.CHDEF_BIG,
}

/// <summary>A call into chdef that did not succeed.</summary>
public sealed class ChdefException : Exception
{
    /// <summary>The ABI status (`CHDEF_ERR_*`).</summary>
    public int Status { get; }

    internal ChdefException(int status, string message)
        : base(message)
    {
        Status = status;
    }
}

/// <summary>
/// One channel of the frame. <see cref="Default"/> is null when the
/// channel states none; <see cref="Min"/> and <see cref="Max"/> are the
/// notation their cells used, empty when unstated.
/// </summary>
public sealed record Channel(
    uint Number,
    ulong At,
    ulong Bytes,
    string Name,
    string Type,
    double Lsb,
    double Offset,
    string Unit,
    ulong? Default,
    string Section,
    string Memo,
    string Var,
    string Format,
    string Min,
    string Max,
    bool Favorite,
    ulong BitCount);

/// <summary>
/// A per-row problem found while reading. Every field a consumer needs to
/// write its own sentence is here; <see cref="Message"/> is English prose
/// whose wording is not part of the contract.
/// </summary>
public sealed record Issue(
    string Code,
    int? Row,
    int? Col,
    uint? Channel,
    int? Bit,
    string? Found,
    string? Used,
    string Message);

/// <summary>One channel's readings from a decoded frame.</summary>
public sealed record Reading(uint Channel, ulong Raw, double Value);

/// <summary>
/// A value for one channel: a physical value, or a raw bit pattern.
/// </summary>
public readonly struct Value
{
    private readonly ChdefValue _inner;

    private Value(ChdefValue inner) => _inner = inner;

    /// <summary>A physical value, converted by the channel's lsb and offset.</summary>
    public static Value Physical(uint channel, double value) =>
        new(new ChdefValue { Channel = channel, Kind = 0, Physical = value });

    /// <summary>A raw bit pattern, written as given.</summary>
    public static Value Raw(uint channel, ulong raw) =>
        new(new ChdefValue { Channel = channel, Kind = 1, Raw = raw });

    internal ChdefValue Inner => _inner;
}

/// <summary>
/// A CH / BF definition set read into a frame layout. Dispose it once.
/// </summary>
public sealed unsafe class Definitions : IDisposable
{
    private nint _layout;
    private readonly List<Channel> _channels;

    /// <summary>The problems found while reading these definitions.</summary>
    public IReadOnlyList<Issue> Issues { get; }

    private Definitions(nint layout, IReadOnlyList<Issue> issues)
    {
        _layout = layout;
        Issues = issues;
        _channels = ReadChannels(layout);
    }

    /// <summary>
    /// The revision of the ABI the loaded native library implements. It
    /// must match what this assembly was built against.
    /// </summary>
    public static uint AbiVersion => Native.chdef_abi_version();

    /// <summary>
    /// Read a CH CSV and an optional BF CSV. Throws
    /// <see cref="ChdefException"/> when the text is not UTF-8 or the CSV
    /// is structurally broken; a merely wrong file loads, with
    /// <see cref="Issues"/>.
    /// </summary>
    public static Definitions Parse(string chCsv, string? bfCsv = null)
    {
        if (AbiVersion != Native.CHDEF_ABI_VERSION)
        {
            throw new ChdefException(
                Native.CHDEF_ERR_HANDLE,
                $"the native library implements ABI version {AbiVersion}, "
                    + $"this assembly was built for {Native.CHDEF_ABI_VERSION}");
        }

        var ch = Encoding.UTF8.GetBytes(chCsv);
        var bf = Encoding.UTF8.GetBytes(bfCsv ?? string.Empty);
        var error = new byte[512];

        nint layout = 0;
        nint issues = 0;
        int status;
        fixed (byte* chPtr = ch)
        fixed (byte* bfPtr = bf)
        fixed (byte* errPtr = error)
        {
            status = Native.chdef_layout_parse(
                chPtr, (nuint)ch.Length,
                bfPtr, (nuint)bf.Length,
                &layout, &issues,
                errPtr, (nuint)error.Length);
        }

        if (status != Native.CHDEF_OK)
        {
            throw new ChdefException(status, Decode(error) is { Length: > 0 } m ? m : $"status {status}");
        }

        var read = ReadIssues(issues);
        Native.chdef_issues_free(issues);
        return new Definitions(layout, read);
    }

    /// <summary>The data length of the frame in bytes.</summary>
    public ulong TotalBytes => Native.chdef_layout_total_bytes(Handle);

    /// <summary>The channels, in the order that fixes their positions.</summary>
    public IReadOnlyList<Channel> Channels => _channels;

    /// <summary>Byte order of every multi-byte channel.</summary>
    public Endian Endian
    {
        set => Check(Native.chdef_layout_set_endian(Handle, (int)value));
    }

    /// <summary>
    /// The maximum byte count of the data part, for
    /// <see cref="CheckCapacity"/>.
    /// </summary>
    public ulong Capacity
    {
        set => Check(Native.chdef_layout_set_capacity(Handle, value));
    }

    /// <summary>
    /// The finding when the frame does not fit the capacity set through
    /// <see cref="Capacity"/>; empty when it fits or none was set.
    /// </summary>
    public IReadOnlyList<Issue> CheckCapacity()
    {
        nint issues = 0;
        Check(Native.chdef_layout_check_capacity(Handle, &issues));
        var read = ReadIssues(issues);
        Native.chdef_issues_free(issues);
        return read;
    }

    /// <summary>
    /// Build a frame. Channels <paramref name="values"/> does not name
    /// take their default.
    /// </summary>
    public byte[] Encode(IEnumerable<Value> values, out IReadOnlyList<Issue> issues)
    {
        var given = values.Select(v => v.Inner).ToArray();
        var frame = new byte[TotalBytes];
        nuint written = 0;
        nint issueHandle = 0;
        int status;

        fixed (ChdefValue* givenPtr = given)
        fixed (byte* framePtr = frame)
        {
            status = Native.chdef_encode(
                Handle, givenPtr, (nuint)given.Length,
                framePtr, (nuint)frame.Length, &written, &issueHandle);
        }

        Check(status);
        issues = ReadIssues(issueHandle);
        Native.chdef_issues_free(issueHandle);
        return written == (nuint)frame.Length ? frame : frame[..(int)written];
    }

    /// <summary>
    /// Read a frame. A channel that overruns a short frame is omitted, and
    /// so is everything after it.
    /// </summary>
    public IReadOnlyList<Reading> Decode(ReadOnlySpan<byte> frame)
    {
        var readings = new ChdefReading[Channels.Count];
        nuint count = 0;
        int status;

        fixed (byte* framePtr = frame)
        fixed (ChdefReading* outPtr = readings)
        {
            status = Native.chdef_decode(
                Handle, framePtr, (nuint)frame.Length,
                outPtr, (nuint)readings.Length, &count);
        }

        Check(status);
        return readings
            .Take((int)count)
            .Select(r => new Reading(r.Channel, r.Raw, r.Value))
            .ToList();
    }

    /// <inheritdoc />
    public void Dispose()
    {
        if (_layout != 0)
        {
            Native.chdef_layout_free(_layout);
            _layout = 0;
        }
    }

    private nint Handle =>
        _layout != 0
            ? _layout
            : throw new ObjectDisposedException(nameof(Definitions));

    private static void Check(int status)
    {
        if (status != Native.CHDEF_OK)
        {
            throw new ChdefException(status, $"chdef returned status {status}");
        }
    }

    private static List<Channel> ReadChannels(nint layout)
    {
        var count = (int)Native.chdef_layout_channel_count(layout);
        var channels = new List<Channel>(count);
        for (var i = 0; i < count; i++)
        {
            ChdefChannel raw;
            Check(Native.chdef_layout_channel_at(layout, (nuint)i, &raw));
            var index = (nuint)i;
            channels.Add(new Channel(
                raw.Number,
                raw.At,
                raw.Bytes,
                Text(layout, index, Native.CHDEF_CHANNEL_NAME),
                Text(layout, index, Native.CHDEF_CHANNEL_TYPE),
                raw.Lsb,
                raw.Offset,
                Text(layout, index, Native.CHDEF_CHANNEL_UNIT),
                raw.DefaultValue < 0 ? null : (ulong)raw.DefaultValue,
                Text(layout, index, Native.CHDEF_CHANNEL_SECTION),
                Text(layout, index, Native.CHDEF_CHANNEL_MEMO),
                Text(layout, index, Native.CHDEF_CHANNEL_VAR),
                Text(layout, index, Native.CHDEF_CHANNEL_FORMAT),
                Text(layout, index, Native.CHDEF_CHANNEL_MIN),
                Text(layout, index, Native.CHDEF_CHANNEL_MAX),
                raw.Favorite != 0,
                raw.BitCount));
        }
        return channels;
    }

    private static List<Issue> ReadIssues(nint handle)
    {
        var count = (int)Native.chdef_issue_count(handle);
        var issues = new List<Issue>(count);
        for (var i = 0; i < count; i++)
        {
            ChdefIssue raw;
            Check(Native.chdef_issue_at(handle, (nuint)i, &raw));
            var index = (nuint)i;
            issues.Add(new Issue(
                IssueText(handle, index, Native.CHDEF_ISSUE_CODE),
                raw.Row < 0 ? null : (int)raw.Row,
                raw.Col < 0 ? null : (int)raw.Col,
                raw.Channel < 0 ? null : (uint)raw.Channel,
                raw.Bit < 0 ? null : (int)raw.Bit,
                Empty(IssueText(handle, index, Native.CHDEF_ISSUE_FOUND)),
                Empty(IssueText(handle, index, Native.CHDEF_ISSUE_USED)),
                IssueText(handle, index, Native.CHDEF_ISSUE_MESSAGE)));
        }
        return issues;

        static string? Empty(string s) => s.Length == 0 ? null : s;
    }

    /// <summary>
    /// The buffer dance the ABI asks for: query the length, then fill.
    /// </summary>
    private static string Text(nint layout, nuint index, int field)
    {
        var needed = (int)Native.chdef_layout_channel_text(layout, index, field, null, 0);
        if (needed == 0)
        {
            return string.Empty;
        }
        var buf = new byte[needed + 1];
        fixed (byte* ptr = buf)
        {
            Native.chdef_layout_channel_text(layout, index, field, ptr, (nuint)buf.Length);
        }
        return Encoding.UTF8.GetString(buf, 0, needed);
    }

    private static string IssueText(nint handle, nuint index, int field)
    {
        var needed = (int)Native.chdef_issue_text(handle, index, field, null, 0);
        if (needed == 0)
        {
            return string.Empty;
        }
        var buf = new byte[needed + 1];
        fixed (byte* ptr = buf)
        {
            Native.chdef_issue_text(handle, index, field, ptr, (nuint)buf.Length);
        }
        return Encoding.UTF8.GetString(buf, 0, needed);
    }

    private static string Decode(byte[] nulTerminated)
    {
        var end = Array.IndexOf(nulTerminated, (byte)0);
        return Encoding.UTF8.GetString(nulTerminated, 0, end < 0 ? nulTerminated.Length : end);
    }
}
