// Derived channels through the managed API (docs/spec/format.md §6):
// sealing, checking, and the coverage a caller computing its own checksum
// needs (ADR-0029).

using Xunit;

namespace Chdef.Tests;

public sealed class DerivedTests
{
    private const string Ch =
        "number,bytes,type,kind,derived,default,name\n"
        + "1,2,UI,const,,0x7E7E,SYNC\n"
        + "2,2,UI,counter,,1,FRAME_NO\n"
        + "3,1,UI,plain,,42,PAYLOAD\n"
        + "4,2,UI,derived,crc16/x25 1..3,,CRC\n";

    private static readonly byte[] Body = [0x7E, 0x7E, 0x01, 0x00, 0x2A];
    private const ushort CrcOver1To3 = 0x9BBF;

    [Fact]
    public void EncodeLeavesTheDerivedChannelAndSealingFillsIt()
    {
        using var defs = Definitions.Parse(Ch);
        var frame = defs.Encode([], out var issues);

        Assert.Empty(issues);
        Assert.Equal(Body, frame[..5]);
        Assert.Equal(new byte[] { 0, 0 }, frame[5..]);

        Assert.Empty(defs.Seal(frame));
        Assert.Equal(BitConverter.GetBytes(CrcOver1To3), frame[5..]);
        Assert.Equal(Body, frame[..5]);
    }

    [Fact]
    public void TheRecipeCellArrivesAsTheFileSpellsIt()
    {
        using var defs = Definitions.Parse(Ch);

        Assert.Equal("derived", defs.Channels[3].Kind);
        Assert.Equal("crc16/x25 1..3", defs.Channels[3].Derived);
        Assert.Equal("", defs.Channels[0].Derived);
    }

    [Fact]
    public void ASealedFrameChecksOutAndAnUnsealedOneDoesNot()
    {
        using var defs = Definitions.Parse(Ch);
        var frame = defs.Encode([], out _);

        var issue = Assert.Single(defs.DerivedMismatches(frame));
        Assert.Equal(IssueCode.DerivedMismatch, issue.Code);
        Assert.Equal(4u, issue.Channel);

        defs.Seal(frame);
        Assert.Empty(defs.DerivedMismatches(frame));
    }

    [Fact]
    public void ACorruptedFrameIsNamed()
    {
        using var defs = Definitions.Parse(Ch);
        var frame = defs.Encode([], out _);
        defs.Seal(frame);

        frame[4] ^= 0xFF;

        Assert.Single(defs.DerivedMismatches(frame));
    }

    [Fact]
    public void ARecipeThisLibraryDoesNotComputeStillSaysWhatItCovers()
    {
        // The escape hatch: the algorithm is the caller's, the coverage is
        // chdef's, and only the second is hard to get right.
        using var defs = Definitions.Parse(Ch.Replace("crc16/x25 1..3", "fletcher16 1..3"));
        var frame = defs.Encode([], out _);

        var issue = Assert.Single(defs.Seal(frame));
        Assert.Equal(IssueCode.DerivedUnknownRecipe, issue.Code);
        Assert.Equal(new byte[] { 0, 0 }, frame[5..]);

        var covered = defs.CoveredBytes(4, frame);
        Assert.Equal(Body, covered);

        // The caller writes its own through the ordinary door.
        ushort mine = 0;
        foreach (var b in covered!)
        {
            mine += b;
        }
        var sealedFrame = defs.Encode([Value.Raw(4, mine)], out _);
        Assert.Equal(BitConverter.GetBytes(mine), sealedFrame[5..]);
    }

    [Fact]
    public void AChannelWithNoCoverageToGiveAnswersWithNothing()
    {
        using var defs = Definitions.Parse(Ch);
        var frame = defs.Encode([], out _);

        Assert.Null(defs.CoveredBytes(1, frame));
        Assert.Null(defs.CoveredBytes(9, frame));
    }

    [Fact]
    public void TheRecipesThisLibraryKnowsAreEnumerable()
    {
        var recipes = Definitions.Recipes();
        Assert.Contains("crc16/x25", recipes);
        Assert.True(recipes.Count >= 6, $"found {recipes.Count}");
    }
}
