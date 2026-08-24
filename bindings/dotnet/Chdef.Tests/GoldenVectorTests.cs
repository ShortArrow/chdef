// The golden vectors of docs/spec/interchange.md §3, run through the C#
// binding.
//
// This is what makes shipping a binding worth the cost (ADR-0022): the
// vectors certify the path the C# project actually takes, not a path
// beside it. The same files certify the crate and the C ABI.

using System.Globalization;
using Xunit;

namespace Chdef.Tests;

public class GoldenVectorTests
{
    public static TheoryData<string> VectorSets()
    {
        var data = new TheoryData<string>();
        foreach (var dir in Directory.GetDirectories(VectorRoot).OrderBy(d => d))
        {
            data.Add(Path.GetFileName(dir));
        }
        return data;
    }

    [Theory]
    [MemberData(nameof(VectorSets))]
    public void EveryVectorHoldsThroughTheBinding(string set)
    {
        var dir = Path.Combine(VectorRoot, set);
        using var defs = Definitions.Parse(
            File.ReadAllText(Path.Combine(dir, "ch.csv")),
            File.ReadAllText(Path.Combine(dir, "bf.csv")));

        var checkedLines = 0;
        var expectedIssues = new List<string>();
        var issueSources = 0;
        var lines = File.ReadAllLines(Path.Combine(dir, "vectors.txt"));
        for (var i = 0; i < lines.Length; i++)
        {
            var line = lines[i].Trim();
            if (line.Length == 0 || line.StartsWith('#'))
            {
                continue;
            }

            var at = $"{set}/vectors.txt:{i + 1} (through the C# binding)";
            var fields = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            switch (fields[0])
            {
                case "B":
                    defs.Endian = fields[1] == "big" ? Endian.Big : Endian.Little;
                    break;
                case "E":
                    CheckEncode(defs, fields[1], fields[2], at);
                    checkedLines++;
                    break;
                case "D":
                    CheckDecode(defs, fields[1], fields[2], at);
                    checkedLines++;
                    break;
                case "L":
                    CheckLayout(defs, fields[1], fields[2], at);
                    checkedLines++;
                    break;
                case "F":
                    CheckBits(defs, fields[1], fields[2], at);
                    checkedLines++;
                    break;
                case "P":
                    // The `P` lines name Issues per source file; the
                    // binding sees the one merged list its parse returns,
                    // so what is contracted here is their union.
                    expectedIssues.AddRange(Declared(fields[2]));
                    issueSources++;
                    break;
                default:
                    Assert.Fail($"{at}: unreadable vector line {line}");
                    break;
            }
        }

        if (issueSources > 0)
        {
            expectedIssues.Sort(StringComparer.Ordinal);
            var actual = defs.Issues
                .Select(issue => $"{issue.Code}:{(issue.Row is { } row ? row.ToString(CultureInfo.InvariantCulture) : "-")}")
                .OrderBy(x => x, StringComparer.Ordinal)
                .ToList();
            Assert.Equal(expectedIssues, actual);
        }

        Assert.True(checkedLines > 0, $"{set}: nothing was checked through the binding");
    }

    /// <summary>
    /// An `F` line: the frame, then `channel:bit=value` for each bit it
    /// names.
    /// </summary>
    private static void CheckBits(Definitions defs, string frameHex, string expected, string at)
    {
        var readings = defs.Decode(Convert.FromHexString(frameHex));
        foreach (var want in expected.Split(';'))
        {
            var (position, value) = Split(want, '=');
            var (number, bit) = Split(position, ':');
            var channel = uint.Parse(number, CultureInfo.InvariantCulture);
            var index = uint.Parse(bit, CultureInfo.InvariantCulture);

            var reading = readings.FirstOrDefault(r => r.Channel == channel);
            Assert.True(reading is not null, $"{at}: channel {channel} is not in the frame");

            var read = reading!.Bits.FirstOrDefault(b => b.Number == index);
            Assert.True(read is not null, $"{at}: bit {index} of channel {channel} is not defined");
            Assert.Equal(value == "1", read!.Value);
        }
    }

    private static (string, string) Split(string text, char at)
    {
        var parts = text.Split(at, 2);
        Assert.Equal(2, parts.Length);
        return (parts[0], parts[1]);
    }

    private static List<string> Declared(string field) =>
        field == "-" ? [] : [.. field.Split(';')];

    private static void CheckEncode(Definitions defs, string values, string expectedHex, string at)
    {
        var frame = defs.Encode(ParseValues(values), out var issues);

        Assert.Empty(issues);
        Assert.Equal(expectedHex, Convert.ToHexString(frame).ToLowerInvariant());
    }

    private static void CheckDecode(Definitions defs, string frameHex, string expected, string at)
    {
        var readings = defs.Decode(Convert.FromHexString(frameHex));
        var wanted = expected.Split(';');

        Assert.Equal(wanted.Length, readings.Count);
        for (var i = 0; i < wanted.Length; i++)
        {
            var parts = wanted[i].Split('=', '/');
            var number = uint.Parse(parts[0], CultureInfo.InvariantCulture);
            var raw = ulong.Parse(parts[1], CultureInfo.InvariantCulture);
            var value = double.Parse(parts[2], CultureInfo.InvariantCulture);

            Assert.Equal(number, readings[i].Channel);
            Assert.Equal(raw, readings[i].Raw);
            Assert.True(
                Math.Abs(readings[i].Value - value) <= 1e-9 * Math.Max(Math.Abs(value), 1.0),
                $"{at}: channel {number} value {readings[i].Value} != {value}");
        }
    }

    private static void CheckLayout(Definitions defs, string total, string positions, string at)
    {
        Assert.Equal(total, defs.TotalBytes.ToString(CultureInfo.InvariantCulture));

        var wanted = positions.Split(';');
        Assert.Equal(wanted.Length, defs.Channels.Count);
        for (var i = 0; i < wanted.Length; i++)
        {
            var ch = defs.Channels[i];
            Assert.Equal(wanted[i], $"{ch.Number}:{ch.At}:{ch.Bytes}");
        }
    }

    private static IEnumerable<Value> ParseValues(string field)
    {
        if (field == "-")
        {
            return Array.Empty<Value>();
        }
        return field.Split(';').Select(pair =>
        {
            var parts = pair.Split('=');
            var channel = uint.Parse(parts[0], CultureInfo.InvariantCulture);
            var text = parts[1];
            return text.StartsWith("0x", StringComparison.OrdinalIgnoreCase)
                ? Value.Raw(channel, Convert.ToUInt64(text[2..], 16))
                : Value.Physical(channel, double.Parse(text, CultureInfo.InvariantCulture));
        });
    }

    /// <summary>
    /// The vectors live with the crate that defines what correct is, so the
    /// binding reads the same files rather than a copy of them.
    /// </summary>
    internal static string VectorRoot
    {
        get
        {
            var dir = AppContext.BaseDirectory;
            while (dir is not null
                   && !Directory.Exists(Path.Combine(dir, "crates", "chdef", "vectors")))
            {
                dir = Path.GetDirectoryName(dir);
            }
            Assert.NotNull(dir);
            return Path.Combine(dir!, "crates", "chdef", "vectors");
        }
    }
}
