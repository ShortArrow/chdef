// The binding itself: handles, strings, statuses and the fields that
// cross. The golden vectors cover the arithmetic (GoldenVectorTests); these
// cover the marshalling, which is the part a vector cannot see.

using Xunit;

namespace Chdef.Tests;

public class BindingTests
{
    private const string Ch =
        "number,bytes,type,name,lsb,offset,unit,default,format,favorite\n"
        + "1,4,UI,Frame,1,0,,,DEC,0\n"
        + "2,2,BF,Status,1,0,,0x0005,HEX,1\n"
        + "3,2,SI,Temp,0.1,-40,degC,,DEC,0\n";

    private const string Bf = "number,bit,name,default\n2,0,alive,\n2,2,fault,0\n";

    [Fact]
    public void TheNativeLibraryImplementsAtLeastTheAbiThisAssemblyNeeds()
    {
        // docs/spec/abi.md §4: symbols are added and never withdrawn, so
        // the check a caller makes is one-sided.
        Assert.True(Definitions.AbiVersion >= 2u, $"ABI version {Definitions.AbiVersion}");
    }

    [Fact]
    public void ADefinitionSetReadsIntoALayout()
    {
        using var defs = Definitions.Parse(Ch, Bf);

        Assert.Equal(8ul, defs.TotalBytes);
        Assert.Equal(3, defs.Channels.Count);
        Assert.Empty(defs.Issues);
    }

    [Fact]
    public void EveryFieldOfAChannelSurvivesTheCrossing()
    {
        using var defs = Definitions.Parse(Ch, Bf);

        var temp = defs.Channels[2];
        Assert.Equal(3u, temp.Number);
        Assert.Equal(6ul, temp.At);
        Assert.Equal(2ul, temp.Bytes);
        Assert.Equal("Temp", temp.Name);
        Assert.Equal("SI", temp.Type);
        Assert.Equal(0.1, temp.Lsb);
        Assert.Equal(-40.0, temp.Offset);
        Assert.Equal("degC", temp.Unit);
        Assert.Null(temp.Default);
        Assert.Equal("DEC", temp.Format);
        Assert.False(temp.Favorite);
        Assert.Empty(temp.Bits);

        var status = defs.Channels[1];
        Assert.Equal("BF", status.Type);
        // 0x0005 with bit 2 cleared by its BF row.
        Assert.Equal(1ul, status.Default);
        Assert.Equal("HEX", status.Format);
        Assert.True(status.Favorite);
        Assert.Equal(2, status.Bits.Count);
    }

    [Fact]
    public void NonAsciiTextCrossesAsUtf8()
    {
        using var japanese = ColumnVocabulary.Japanese();
        using var defs = Definitions.Parse("番号,メッセージ名称,単位\n1,圧力,kPa\n", null, japanese);

        Assert.Equal("圧力", defs.Channels[0].Name);
        Assert.Equal("kPa", defs.Channels[0].Unit);
    }

    [Fact]
    public void AFrameEncodesAndDecodes()
    {
        using var defs = Definitions.Parse(Ch, Bf);

        var frame = defs.Encode(
            new[] { Value.Physical(1, 7), Value.Physical(3, -12.3) },
            out var issues);

        Assert.Empty(issues);
        Assert.Equal(8, frame.Length);

        var readings = defs.Decode(frame);
        Assert.Equal(3, readings.Count);
        Assert.Equal(7.0, readings[0].Value);
        Assert.Equal(1ul, readings[1].Raw);
        Assert.True(Math.Abs(readings[2].Value - -12.3) < 1e-9);
    }

    [Fact]
    public void AShortFrameDropsTheChannelsThatOverrunIt()
    {
        using var defs = Definitions.Parse(Ch, Bf);

        var readings = defs.Decode(new byte[6]);

        Assert.Equal(2, readings.Count);
    }

    [Fact]
    public void ByteOrderIsTheLayoutsToSet()
    {
        using var defs = Definitions.Parse(Ch, Bf);
        defs.Endian = Endian.Big;

        var frame = defs.Encode(new[] { Value.Raw(1, 1) }, out _);

        Assert.Equal(new byte[] { 0, 0, 0, 1 }, frame[..4]);
    }

    [Fact]
    public void AnIssueCrossesWithItsCodeAndItsValues()
    {
        using var defs = Definitions.Parse("number,bytes,name\n1,99,a\n");

        var issue = Assert.Single(defs.Issues);
        Assert.Equal("bytes_out_of_range", issue.Code);
        Assert.Equal(0, issue.Row);
        Assert.Equal(1u, issue.Channel);
        Assert.Null(issue.Bit);
        Assert.Equal("99", issue.Found);
        Assert.Equal("8", issue.Used);
        Assert.NotEmpty(issue.Message);
    }

    [Fact]
    public void AnIssueThatNamesABitNamesBothHalves()
    {
        // Channel 1 is UI, so its BF row has no bitfield parent.
        using var defs = Definitions.Parse("number,bytes,type,name\n1,2,UI,a\n", "number,bit\n1,0\n");

        var issue = Assert.Single(defs.Issues, i => i.Code == "bf_parent_not_bitfield");
        Assert.Equal(1u, issue.Channel);
        Assert.Equal(0, issue.Bit);
        Assert.Null(issue.Row);
    }

    [Fact]
    public void AStructurallyBrokenFileThrowsWithTheReason()
    {
        var broken = Assert.Throws<ChdefException>(
            () => Definitions.Parse("number,name\n1,\"never closed\n2,b\n"));

        Assert.Equal(-6, broken.Status);
        Assert.Contains("quoted cell", broken.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void ACapacityIsCheckedOnlyWhenStated()
    {
        using var defs = Definitions.Parse(Ch, Bf);
        Assert.Empty(defs.LimitsExceeded());

        defs.Capacity = 4;
        var issue = Assert.Single(defs.LimitsExceeded());
        Assert.Equal("layout_exceeds_capacity", issue.Code);
        Assert.Equal("8", issue.Found);
        Assert.Equal("4", issue.Used);
    }

    [Fact]
    public void EncodeReportsAValueItCouldNotPlace()
    {
        using var defs = Definitions.Parse(Ch, Bf);

        defs.Encode(new[] { Value.Raw(9, 1) }, out var issues);

        Assert.Equal("encode_unknown_channel", Assert.Single(issues).Code);
    }

    [Fact]
    public void UsingADisposedDefinitionSetThrowsRatherThanReadingFreedMemory()
    {
        var defs = Definitions.Parse(Ch, Bf);
        defs.Dispose();

        Assert.Throws<ObjectDisposedException>(() => defs.TotalBytes);
    }

    [Fact]
    public void DisposingTwiceIsHarmless()
    {
        var defs = Definitions.Parse(Ch, Bf);

        defs.Dispose();
        defs.Dispose();
    }
}
