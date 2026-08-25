// The safe surface over Native.cs: handles that dispose themselves, the
// two-call buffer dance for every string done once, and a status turned
// into an exception. Nothing here reaches past the ABI.

using System.Text;

namespace Chdef;

/// <summary>Byte order of every multi-byte channel of the frame.</summary>
public enum Endian
{
    /// <summary>Least significant byte first.</summary>
    Little = Native.CHDEF_LITTLE,

    /// <summary>Most significant byte first.</summary>
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
    string Kind,
    string Derived,
    IReadOnlyList<Bit> Bits);

/// <summary>
/// One named bit of a channel. <see cref="Default"/> is null when the BF
/// row names none and the bit keeps the parent channel's.
/// </summary>
public sealed record Bit(
    uint Channel,
    uint Number,
    string Name,
    string Memo,
    int? Default);

/// <summary>One named bit of a decoded frame, and whether it is set.</summary>
public sealed record BitReading(uint Channel, uint Number, string Name, bool Value);

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

/// <summary>
/// One channel's readings from a decoded frame. <see cref="Bits"/> is
/// empty for a channel with no bits named, whatever its type.
/// </summary>
public sealed record Reading(
    uint Channel,
    ulong Raw,
    double Value,
    IReadOnlyList<BitReading> Bits);

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

    /// <summary>
    /// Read the text form of a value (docs/spec/format.md §3): a leading
    /// <c>0x</c> or <c>0X</c> is a raw bit pattern, anything else a
    /// physical value. Throws <see cref="ChdefException"/> for text that
    /// denotes no value.
    /// </summary>
    public static unsafe Value Parse(string text, uint channel)
    {
        if (!TryParse(text, channel, out var value))
        {
            throw new ChdefException(
                Native.CHDEF_ERR_VALUE, $"\"{text}\" denotes no value");
        }
        return value;
    }

    /// <summary>
    /// <see cref="Parse"/> without the exception, for a text box the user
    /// is still typing into.
    /// </summary>
    public static unsafe bool TryParse(string text, uint channel, out Value value)
    {
        var utf8 = Encoding.UTF8.GetBytes(text);
        ChdefValue read;
        int status;
        fixed (byte* ptr = utf8)
        {
            status = Native.chdef_value_parse(ptr, (nuint)utf8.Length, channel, &read);
        }
        value = new Value(read);
        return status == Native.CHDEF_OK;
    }

    /// <summary>The channel this value is for.</summary>
    public uint Channel => _inner.Channel;

    /// <summary>Whether this is a raw bit pattern rather than a physical value.</summary>
    public bool IsRaw => _inner.Kind == 1;

    /// <summary>The physical value; meaningful when <see cref="IsRaw"/> is false.</summary>
    public double PhysicalValue => _inner.Physical;

    /// <summary>The raw bit pattern; meaningful when <see cref="IsRaw"/> is true.</summary>
    public ulong RawValue => _inner.Raw;

    internal ChdefValue Inner => _inner;

    internal static Value From(ChdefValue inner) => new(inner);
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
    public static Definitions Parse(string chCsv, string? bfCsv = null) =>
        Parse(chCsv, bfCsv, null);

    /// <summary>
    /// <see cref="Parse(string, string?)"/>, reading both headers with
    /// <paramref name="vocabulary"/> on top of the canonical column names
    /// (docs/spec/format.md §2). A null vocabulary is the empty one.
    /// </summary>
    public static Definitions Parse(
        string chCsv,
        string? bfCsv,
        ColumnVocabulary? vocabulary)
    {
        if (AbiVersion < Native.CHDEF_ABI_VERSION)
        {
            throw new ChdefException(
                Native.CHDEF_ERR_HANDLE,
                $"the native library implements ABI version {AbiVersion}, "
                    + $"this assembly needs at least {Native.CHDEF_ABI_VERSION}");
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
            status = Native.chdef_layout_parse_with(
                chPtr, (nuint)ch.Length,
                bfPtr, (nuint)bf.Length,
                vocabulary?.Handle ?? 0,
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
    /// <see cref="LimitsExceeded"/>.
    /// </summary>
    public ulong Capacity
    {
        set => Check(Native.chdef_layout_set_capacity(Handle, value));
    }

    /// <summary>
    /// The maximum number of channels the port accepts — the limit a byte
    /// count cannot express. Same terms as <see cref="Capacity"/>.
    /// </summary>
    public ulong ChannelCapacity
    {
        set => Check(Native.chdef_layout_set_channel_capacity(Handle, value));
    }

    /// <summary>
    /// The findings when the layout does not fit the limits set through
    /// <see cref="Capacity"/> and <see cref="ChannelCapacity"/>; empty when
    /// it fits or none was set.
    /// </summary>
    public IReadOnlyList<Issue> LimitsExceeded()
    {
        nint issues = 0;
        Check(Native.chdef_layout_limits_exceeded(Handle, &issues));
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
        var bits = DecodeBits(frame);
        return readings
            .Take((int)count)
            .Select(r => new Reading(
                r.Channel,
                r.Raw,
                r.Value,
                bits.TryGetValue(r.Channel, out var of) ? of : Array.Empty<BitReading>()))
            .ToList();
    }

    /// <summary>
    /// Every named bit of a frame in one pass, grouped by channel — what
    /// <see cref="Decode(ReadOnlySpan{byte})"/> hands to each <see cref="Reading"/>.
    /// </summary>
    private Dictionary<uint, IReadOnlyList<BitReading>> DecodeBits(ReadOnlySpan<byte> frame)
    {
        var total = (int)Native.chdef_layout_bit_total(Handle);
        var grouped = new Dictionary<uint, IReadOnlyList<BitReading>>();
        if (total == 0)
        {
            return grouped;
        }

        // The names live on the channels this layout already read; the
        // ABI carries numbers, so the join happens here rather than in
        // every caller.
        var named = _channels
            .SelectMany(c => c.Bits)
            .ToDictionary(b => (b.Channel, b.Number), b => b.Name);

        var bits = new ChdefBitReading[total];
        nuint count = 0;
        int status;
        fixed (byte* framePtr = frame)
        fixed (ChdefBitReading* outPtr = bits)
        {
            status = Native.chdef_decode_bits(
                Handle, framePtr, (nuint)frame.Length, outPtr, (nuint)bits.Length, &count);
        }
        Check(status);

        foreach (var bit in bits.Take((int)count))
        {
            if (!grouped.TryGetValue(bit.Channel, out var of))
            {
                of = new List<BitReading>();
                grouped[bit.Channel] = of;
            }
            ((List<BitReading>)of).Add(new BitReading(
                bit.Channel,
                bit.Bit,
                named.TryGetValue((bit.Channel, bit.Bit), out var name) ? name : string.Empty,
                bit.Value != 0));
        }
        return grouped;
    }

    /// <summary>
    /// Which of the two readings the channel's <c>format</c> column
    /// selects (docs/spec/conversion.md §7): the physical value for
    /// <c>DEC</c>, the raw one for <c>HEX</c>. It affects no conversion.
    /// </summary>
    public Value Displayed(int channelIndex, ulong raw)
    {
        ChdefValue value;
        Check(Native.chdef_layout_channel_displayed(
            Handle, (nuint)channelIndex, raw, &value));
        return Value.From(value);
    }

    /// <summary>
    /// The default text form of a reading — the physical value with the
    /// channel's unit, or the raw one in hexadecimal padded to the
    /// channel's width. A consumer with its own digit counts, separators
    /// or colours writes its own instead.
    /// </summary>
    public string Render(int channelIndex, ulong raw) =>
        TextBuffer.Read((buf, cap) =>
            Native.chdef_layout_channel_render(Handle, (nuint)channelIndex, raw, buf, cap));

    /// <summary>
    /// Which of <paramref name="values"/> fall outside their channel's
    /// declared range (docs/spec/conversion.md §8). Nothing is changed and
    /// nothing is remembered: <see cref="Encode"/> behaves the same
    /// whether this was called or not.
    /// </summary>
    public IReadOnlyList<Issue> ValuesOutOfRange(IEnumerable<Value> values)
    {
        var given = values.Select(v => v.Inner).ToArray();
        nint handle = 0;
        int status;
        fixed (ChdefValue* ptr = given)
        {
            status = Native.chdef_values_out_of_range(Handle, ptr, (nuint)given.Length, &handle);
        }
        Check(status);
        var read = ReadIssues(handle);
        Native.chdef_issues_free(handle);
        return read;
    }

    /// <summary>
    /// Which of <paramref name="readings"/> fall outside their channel's
    /// declared range — the same question as <see cref="ValuesOutOfRange"/>,
    /// asked of a frame that has arrived.
    /// </summary>
    public IReadOnlyList<Issue> ReadingsOutOfRange(IEnumerable<Reading> readings)
    {
        var given = readings
            .Select(r => new ChdefReading { Channel = r.Channel, Raw = r.Raw, Value = r.Value })
            .ToArray();
        nint handle = 0;
        int status;
        fixed (ChdefReading* ptr = given)
        {
            status = Native.chdef_readings_out_of_range(Handle, ptr, (nuint)given.Length, &handle);
        }
        Check(status);
        var read = ReadIssues(handle);
        Native.chdef_issues_free(handle);
        return read;
    }

    /// <summary>
    /// The derivation recipes this library knows by name. The set can
    /// grow, and a recipe naming something outside it is still read — its
    /// coverage is available through <see cref="CoveredBytes"/>.
    /// </summary>
    public static IReadOnlyList<string> Recipes()
    {
        var count = (int)Native.chdef_recipe_count();
        var names = new List<string>(count);
        for (var i = 0; i < count; i++)
        {
            var index = (nuint)i;
            names.Add(TextBuffer.Read((buf, cap) => Native.chdef_recipe_name(index, buf, cap)));
        }
        return names;
    }

    /// <summary>
    /// Fill every derived channel of <paramref name="frame"/>
    /// (docs/spec/format.md §6). <see cref="Encode"/> never does this:
    /// sealing is a call of its own, made once after every other value is
    /// in place. Nothing is written for a channel that is reported.
    /// </summary>
    public IReadOnlyList<Issue> Seal(byte[] frame)
    {
        nint handle = 0;
        int status;
        fixed (byte* ptr = frame)
        {
            status = Native.chdef_seal(Handle, ptr, (nuint)frame.Length, &handle);
        }
        Check(status);
        var read = ReadIssues(handle);
        Native.chdef_issues_free(handle);
        return read;
    }

    /// <summary>
    /// Which derived channels of <paramref name="frame"/> disagree with
    /// their recipe — the check a receiver makes. Nothing is changed.
    /// </summary>
    public IReadOnlyList<Issue> DerivedMismatches(ReadOnlySpan<byte> frame)
    {
        nint handle = 0;
        int status;
        fixed (byte* ptr = frame)
        {
            status = Native.chdef_derived_mismatches(Handle, ptr, (nuint)frame.Length, &handle);
        }
        Check(status);
        var read = ReadIssues(handle);
        Native.chdef_issues_free(handle);
        return read;
    }

    /// <summary>
    /// The bytes a derived channel's recipe covers, in the order it covers
    /// them — the storey below <see cref="Seal"/>. A device whose checksum
    /// chdef does not compute still says which bytes it covers, so a
    /// caller runs its own over exactly those and writes the result
    /// through <see cref="Encode"/>. Null when the channel is not derived,
    /// its recipe was unreadable, or the frame is too short.
    /// </summary>
    public byte[]? CoveredBytes(uint channel, ReadOnlySpan<byte> frame)
    {
        nuint needed = 0;
        int status;
        fixed (byte* ptr = frame)
        {
            status = Native.chdef_covered_bytes(
                Handle, channel, ptr, (nuint)frame.Length, null, 0, &needed);
        }
        if (status == Native.CHDEF_ERR_INDEX)
        {
            return null;
        }

        var covered = new byte[(int)needed];
        fixed (byte* ptr = frame)
        fixed (byte* outPtr = covered)
        {
            status = Native.chdef_covered_bytes(
                Handle, channel, ptr, (nuint)frame.Length, outPtr, needed, &needed);
        }
        Check(status);
        return covered;
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
                Text(layout, index, Native.CHDEF_CHANNEL_KIND),
                Text(layout, index, Native.CHDEF_CHANNEL_DERIVED),
                ReadBits(layout, index, (int)raw.BitCount)));
        }
        return channels;
    }

    private static List<Bit> ReadBits(nint layout, nuint channelIndex, int count)
    {
        var bits = new List<Bit>(count);
        for (var i = 0; i < count; i++)
        {
            ChdefBit raw;
            var bitIndex = (nuint)i;
            Check(Native.chdef_layout_bit_at(layout, channelIndex, bitIndex, &raw));
            bits.Add(new Bit(
                raw.Channel,
                raw.Bit,
                BitText(layout, channelIndex, bitIndex, Native.CHDEF_BIT_NAME),
                BitText(layout, channelIndex, bitIndex, Native.CHDEF_BIT_MEMO),
                raw.DefaultValue < 0 ? null : raw.DefaultValue));
        }
        return bits;
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

    private static string Text(nint layout, nuint index, int field) =>
        TextBuffer.Read((buf, cap) =>
            Native.chdef_layout_channel_text(layout, index, field, buf, cap));

    private static string BitText(nint layout, nuint channelIndex, nuint bitIndex, int field) =>
        TextBuffer.Read((buf, cap) =>
            Native.chdef_layout_bit_text(layout, channelIndex, bitIndex, field, buf, cap));

    private static string IssueText(nint handle, nuint index, int field) =>
        TextBuffer.Read((buf, cap) => Native.chdef_issue_text(handle, index, field, buf, cap));

    private static string Decode(byte[] nulTerminated)
    {
        var end = Array.IndexOf(nulTerminated, (byte)0);
        return Encoding.UTF8.GetString(nulTerminated, 0, end < 0 ? nulTerminated.Length : end);
    }
}
