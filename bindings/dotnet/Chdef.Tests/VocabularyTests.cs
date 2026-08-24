// The column vocabulary through the managed API (ADR-0024): a vocabulary
// chdef ships and one a caller builds behave identically, and neither is a
// language the binding knows.

using Xunit;

namespace Chdef.Tests;

public sealed class VocabularyTests
{
    private const string JaCh = "番号,バイト数,メッセージ名称,型\n7,4,Frame,UI32\n";
    private const string DeCh = "Nummer,Bytes,Bezeichnung,Typ\n7,4,Frame,UI32\n";

    private static ColumnVocabulary German() =>
        ColumnVocabulary.Create()
            .Ch("Nummer", ChColumn.Number)
            .Ch("Bytes", ChColumn.Bytes)
            .Ch("Bezeichnung", ChColumn.Name)
            .Ch("Typ", ChColumn.Type);

    [Fact]
    public void TheEnumMatchesTheCanonicalNamesTheAbiReports()
    {
        // The binding must not hold a second list of columns: the enum is
        // checked against the one the ABI reports, in order.
        Assert.Equal(
            ColumnVocabulary.ChColumnNames(),
            Enum.GetValues<ChColumn>().Select(c => c.ToString().ToLowerInvariant()).ToList());
        Assert.Equal(
            ColumnVocabulary.BfColumnNames(),
            Enum.GetValues<BfColumn>().Select(c => c.ToString().ToLowerInvariant()).ToList());
    }

    [Fact]
    public void NoVocabularyReadsTheCanonicalNamesAlone()
    {
        using var canonical = Definitions.Parse("number,bytes,name,type\n7,4,Frame,UI32\n");
        Assert.Equal(7u, canonical.Channels[0].Number);
        Assert.Empty(canonical.Issues);

        using var japanese = Definitions.Parse(JaCh);
        Assert.Contains(japanese.Issues, i => i.Code == "header_assumed");
    }

    [Fact]
    public void ALanguageTheBindingNeverHeardOfReadsLikeTheOneItShips()
    {
        using var german = German();
        using var byHand = Definitions.Parse(DeCh, null, german);

        using var japanese = ColumnVocabulary.Japanese();
        using var shipped = Definitions.Parse(JaCh, null, japanese);

        Assert.Empty(byHand.Issues);
        Assert.Empty(shipped.Issues);
        Assert.Equal(shipped.Channels[0].Number, byHand.Channels[0].Number);
        Assert.Equal(shipped.Channels[0].Name, byHand.Channels[0].Name);
        Assert.Equal(shipped.Channels[0].Bytes, byHand.Channels[0].Bytes);
    }

    [Fact]
    public void TheCanonicalNamesStayReadableUnderAnyVocabulary()
    {
        using var german = German();
        using var defs = Definitions.Parse("number,Bytes,name\n7,4,Frame\n", null, german);
        Assert.Empty(defs.Issues);
        Assert.Equal(7u, defs.Channels[0].Number);
        Assert.Equal("Frame", defs.Channels[0].Name);
    }

    [Fact]
    public void AVocabularyCannotReassignACanonicalName()
    {
        using var mischief = ColumnVocabulary.Create().Ch("number", ChColumn.Bytes);
        using var defs = Definitions.Parse("number,bytes\n7,4\n", null, mischief);
        Assert.Equal(7u, defs.Channels[0].Number);
        Assert.Equal(4ul, defs.Channels[0].Bytes);
    }

    [Fact]
    public void AUsedVocabularyRefusesToBeUsedAfterDisposal()
    {
        var vocabulary = ColumnVocabulary.Create();
        vocabulary.Dispose();
        Assert.Throws<ObjectDisposedException>(() => vocabulary.Ch("x", ChColumn.Number));
        vocabulary.Dispose();
    }

    [Fact]
    public void TheShippedVocabularyReadsAWholeJapaneseDefinitionSet()
    {
        const string ch =
            "番号,バイト数,メッセージ名称,型,LSB,オフセット,単位,値(デフォルト),表示形式\n"
            + "1,2,ステータス,BF,1,0,,5,HEX\n"
            + "2,1,速度,UI8,1,10,km/h,,DEC\n";
        const string bf = "番号,BIT番号,メッセージ名称,値(デフォルト),備考\n1,0,有効,1,固定\n";

        using var japanese = ColumnVocabulary.Japanese();
        using var defs = Definitions.Parse(ch, bf, japanese);

        Assert.Empty(defs.Issues);
        Assert.Equal(2, defs.Channels.Count);
        Assert.Equal("ステータス", defs.Channels[0].Name);
        Assert.Equal("BF", defs.Channels[0].Type);
        Assert.Single(defs.Channels[0].Bits);
        Assert.Equal("有効", defs.Channels[0].Bits[0].Name);
        Assert.Equal("固定", defs.Channels[0].Bits[0].Memo);
        Assert.Equal("km/h", defs.Channels[1].Unit);
        Assert.Equal("HEX", defs.Channels[0].Format);
    }
}
