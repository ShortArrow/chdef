// Converting one value either way through the managed API, and asking
// whether a width holds it (docs/spec/conversion.md §1–§2). Encode and
// Decode carry the same rules for a whole frame; a consumer showing one
// cell wants them for one number.
//
// The same sentences crates/chdef-capi/tests/spec_abi_conversion.rs
// stands on, asked through the managed API instead.

using Xunit;

namespace Chdef.Tests;

public sealed class ConversionTests
{
    private const string Ch =
        "number,bytes,type,lsb,offset\n"
        + "1,1,UI,1,0\n"
        + "2,1,SI,0.5,-10\n"
        + "3,2,UI,0,0\n";

    [Fact]
    public void APhysicalValueRoundsHalfAwayFromZero()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Equal(1ul, defs.ToRaw(0, 0.5));
        Assert.Equal(3ul, defs.ToRaw(0, 2.5));
        Assert.Equal(0xFFul, defs.ToRaw(1, -10.25));
    }

    [Fact]
    public void AValueTheWidthCannotHoldIsClampedToIt()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Equal(255ul, defs.ToRaw(0, 300));
        Assert.Equal(0ul, defs.ToRaw(0, -3));
    }

    [Fact]
    public void ANegativeSignedValueBecomesItsTwosComplementPattern()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Equal(0xECul, defs.ToRaw(1, -20));
    }

    [Fact]
    public void AZeroLsbCountsAsOne()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Equal(7ul, defs.ToRaw(2, 7));
    }

    [Fact]
    public void AValueThatCannotBeConvertedIsNull()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Null(defs.ToRaw(0, double.NaN));
        Assert.Null(defs.ToRaw(0, double.PositiveInfinity));
    }

    [Fact]
    public void ARawPatternIsScaledAndOffset()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Equal(7.0, defs.ToValue(0, 7));
        Assert.Equal(-20.0, defs.ToValue(1, 0xEC));
        Assert.Equal(5.0, defs.ToValue(2, 5));
    }

    [Fact]
    public void BitsBeyondTheWidthAreIgnored()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Equal(7.0, defs.ToValue(0, 0x107));
    }

    [Fact]
    public void TheTwoConversionsAgreeWithEncodeAndDecode()
    {
        using var defs = Definitions.Parse(Ch);
        var frame = defs.Encode([Value.Physical(2, -20)], out _);
        Assert.Equal(defs.ToRaw(1, -20), (ulong?)frame[1]);
        Assert.Equal(defs.Decode(frame)[1].Value, defs.ToValue(1, frame[1]));
    }

    [Fact]
    public void WhetherTheWidthHoldsAValueIsAnsweredWithoutConvertingIt()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.True(defs.FitsWidth(0, 255));
        Assert.False(defs.FitsWidth(0, 256));
        Assert.False(defs.FitsWidth(0, -1));
        Assert.False(defs.FitsWidth(0, double.NaN));
    }

    [Fact]
    public void AnIndexOutsideTheLayoutThrows()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Equal(-2, Assert.Throws<ChdefException>(() => defs.ToRaw(9, 1)).Status);
        Assert.Equal(-2, Assert.Throws<ChdefException>(() => defs.ToValue(9, 1)).Status);
        Assert.Equal(-2, Assert.Throws<ChdefException>(() => defs.FitsWidth(9, 1)).Status);
        Assert.Equal(-2, Assert.Throws<ChdefException>(() => defs.RangeOf(9)).Status);
    }
}
