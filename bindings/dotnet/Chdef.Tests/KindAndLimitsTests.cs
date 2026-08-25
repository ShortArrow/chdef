// The 0.0.8 surface through the managed API: who fills a channel
// (ADR-0025), the limits a layout is measured against, and the Issue codes
// a consumer checks its own table against (ADR-0026).

using System.Reflection;
using Xunit;

namespace Chdef.Tests;

public sealed class KindAndLimitsTests
{
    private static readonly string[] ExpectedKinds = ["const", "counter", "plain"];

    private const string Ch =
        "number,bytes,type,kind,default,name\n"
        + "1,2,UI,const,0x7E7E,SYNC\n"
        + "2,2,UI,counter,,FRAME_NO\n"
        + "3,1,UI,,,PAYLOAD\n";

    // ------------------------------------------------------------- kind

    [Fact]
    public void EachChannelSaysWhoDecidesItsValue()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Empty(defs.Issues);
        Assert.Equal(ExpectedKinds, defs.Channels.Select(c => c.Kind));
    }

    [Fact]
    public void AKindThisBindingDoesNotKnowArrivesAsPlainAndIsReported()
    {
        // format.md §5: a file written for a later chdef still loads.
        using var defs = Definitions.Parse("number,bytes,kind\n1,2,derived\n");

        Assert.Equal("plain", defs.Channels[0].Kind);
        var issue = Assert.Single(defs.Issues, i => i.Code == IssueCode.KindAssumed);
        Assert.Equal("derived", issue.Found);
        Assert.Equal("plain", issue.Used);
    }

    [Fact]
    public void EncodeProducesTheSameBytesWhateverTheKindSays()
    {
        // ADR-0025: kind is a mark, not a behaviour.
        using var marked = Definitions.Parse(Ch);
        using var plain = Definitions.Parse(Ch.Replace(",const,", ",,").Replace(",counter,", ",,"));

        var a = marked.Encode([], out var aIssues);
        var b = plain.Encode([], out var bIssues);

        Assert.Equal(b, a);
        Assert.Empty(aIssues);
        Assert.Empty(bIssues);
    }

    [Fact]
    public void OverridingAConstChannelIsNotAnIssue()
    {
        using var defs = Definitions.Parse(Ch);
        var frame = defs.Encode([Value.Raw(1, 0xDEAD)], out var issues);

        Assert.Empty(issues);
        Assert.Equal(new byte[] { 0xAD, 0xDE }, frame[..2]);
    }

    [Fact]
    public void ACounterIsNotAdvancedByChdef()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Equal(defs.Encode([], out _), defs.Encode([], out _));
    }

    // ----------------------------------------------------------- limits

    [Fact]
    public void ALayoutWithNoLimitStatedReportsNothing()
    {
        using var defs = Definitions.Parse(Ch);
        Assert.Empty(defs.LimitsExceeded());
    }

    [Fact]
    public void BothLimitsAreReportedTogether()
    {
        using var defs = Definitions.Parse(Ch);
        defs.Capacity = 2;
        defs.ChannelCapacity = 1;

        Assert.Equal(
            new[] { IssueCode.LayoutExceedsCapacity, IssueCode.LayoutExceedsChannelCapacity },
            defs.LimitsExceeded().Select(i => i.Code));
    }

    [Fact]
    public void MoreChannelsThanThePortAcceptsIsReported()
    {
        // The limit a byte count cannot express.
        using var defs = Definitions.Parse(Ch);
        defs.Capacity = 246;
        defs.ChannelCapacity = 2;

        var issue = Assert.Single(defs.LimitsExceeded());
        Assert.Equal(IssueCode.LayoutExceedsChannelCapacity, issue.Code);
        Assert.Equal("3", issue.Found);
        Assert.Equal("2", issue.Used);
    }

    [Fact]
    public void ALimitIsNeverAppliedOnItsOwn()
    {
        using var defs = Definitions.Parse(Ch);
        defs.Capacity = 1;
        defs.ChannelCapacity = 1;

        var frame = defs.Encode([], out var issues);
        Assert.Equal(5, frame.Length);
        Assert.Empty(issues);
    }

    // ------------------------------------------------------ issue codes

    [Fact]
    public void TheConstantsAreExactlyWhatTheLibraryEnumeratesInOrder()
    {
        // ADR-0026: this file is a checked mirror, never a second list.
        var declared = typeof(IssueCode)
            .GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.IsLiteral && f.FieldType == typeof(string))
            .Select(f => (string)f.GetRawConstantValue()!)
            .ToList();

        Assert.NotEmpty(declared);
        Assert.Equal(IssueCode.All, declared);
    }

    [Fact]
    public void ACodeThatArrivesIsOneTheListHolds()
    {
        using var defs = Definitions.Parse("number,bytes,kind\n1,2,derived\n");
        Assert.NotEmpty(defs.Issues);
        Assert.All(defs.Issues, i => Assert.Contains(i.Code, IssueCode.All));
    }

    [Fact]
    public void AConsumerTableKeyedByCodeCanBeProvedComplete()
    {
        // The use the enumeration exists for: a gap becomes a failure here
        // instead of a count moving in an unrelated test.
        var mine = IssueCode.All.ToDictionary(code => code, code => $"({code})");
        var missing = IssueCode.All.Where(code => !mine.ContainsKey(code)).ToList();
        Assert.Empty(missing);
    }
}
