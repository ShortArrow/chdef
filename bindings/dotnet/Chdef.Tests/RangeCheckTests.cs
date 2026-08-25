// Asking whether a value is inside its channel's declared range
// (docs/spec/conversion.md §8), through the managed API.
//
// The ask is a call and not a mode, so Encode and Decode behave the same
// whether it was made or not.

using Xunit;

namespace Chdef.Tests;

public sealed class RangeCheckTests
{
    private const string Ch =
        "number,bytes,type,lsb,min,max,default\n"
        + "1,2,UI,1,0,100,50\n"
        + "2,2,UI,1,,,7\n";

    [Fact]
    public void AValueOutsideItsRangeIsNamedWithTheBoundItCrossed()
    {
        using var defs = Definitions.Parse(Ch);

        var issue = Assert.Single(defs.ValuesOutOfRange([Value.Physical(1, 150)]));
        Assert.Equal(IssueCode.ValueOutOfRange, issue.Code);
        Assert.Equal(1u, issue.Channel);
        Assert.Equal("150", issue.Found);
        Assert.Equal("100", issue.Used);
    }

    [Fact]
    public void AValueInsideItsRangeIsNotNamed()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Empty(defs.ValuesOutOfRange([
            Value.Physical(1, 0),
            Value.Physical(1, 100),
            Value.Physical(2, 1e9),
        ]));
    }

    [Fact]
    public void ARawValueIsJudgedByWhatItMeans()
    {
        using var defs = Definitions.Parse(Ch);
        var issue = Assert.Single(defs.ValuesOutOfRange([Value.Raw(1, 150)]));
        Assert.Equal("150", issue.Found);
    }

    [Fact]
    public void AReadingOutsideItsRangeIsNamed()
    {
        using var defs = Definitions.Parse(Ch);
        var frame = defs.Encode([Value.Physical(1, 150)], out _);

        var issue = Assert.Single(defs.ReadingsOutOfRange(defs.Decode(frame)));
        Assert.Equal(IssueCode.ValueOutOfRange, issue.Code);
        Assert.Equal(1u, issue.Channel);
        Assert.Equal("100", issue.Used);
    }

    [Fact]
    public void AFrameWhoseReadingsAllFitIsQuiet()
    {
        using var defs = Definitions.Parse(Ch);
        var frame = defs.Encode([], out _);
        Assert.Empty(defs.ReadingsOutOfRange(defs.Decode(frame)));
    }

    [Fact]
    public void AskingChangesNothingAboutWhatIsWritten()
    {
        using var defs = Definitions.Parse(Ch);
        Value[] values = [Value.Physical(1, 150)];

        var before = defs.Encode(values, out var beforeIssues);
        Assert.NotEmpty(defs.ValuesOutOfRange(values));
        var after = defs.Encode(values, out var afterIssues);

        Assert.Equal(before, after);
        Assert.Empty(beforeIssues);
        Assert.Empty(afterIssues);
    }

    [Fact]
    public void AnEmptyAskIsNotAnError()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Empty(defs.ValuesOutOfRange([]));
        Assert.Empty(defs.ReadingsOutOfRange([]));
    }
}
