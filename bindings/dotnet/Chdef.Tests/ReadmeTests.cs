// The examples on the NuGet front page, as tests.
//
// C# has no doctest — an XML <example> block is a string and nothing
// compiles it — so the examples live here, where they are compiled and
// run, and a checker in the Rust workspace
// (crates/chdef-capi/tests/dotnet_readme.rs) fails the build if the page
// and this file disagree. The same discipline the header and the P/Invoke
// declarations already use.
//
// Each method body below appears verbatim in bindings/dotnet/Chdef/README.md.

// CA1861 wants a constant array hoisted to a field. These are the
// examples the readme shows, and an example that reaches out of itself for
// a field is a worse example; the rule guards hot paths, which a test is
// not.
#pragma warning disable CA1861

using Xunit;

namespace Chdef.Tests;

public sealed class ReadmeTests
{
    [Fact]
    public void ReadingAFrame()
    {
        const string ch =
            "number,bytes,type,name,lsb,offset,unit\n"
            + "1,2,UI,speed,0.5,0,km/h\n"
            + "2,1,BF,status,1,0,\n";
        const string bf = "number,bit,name\n2,0,ready\n2,1,fault\n";

        using var defs = Definitions.Parse(ch, bf);
        Assert.Empty(defs.Issues);

        // 0x0040 little-endian is raw 64, which is 32 km/h at lsb 0.5.
        var readings = defs.Decode(new byte[] { 0x40, 0x00, 0b01 });

        Assert.Equal(32.0, readings[0].Value);
        Assert.Equal(new[] { ("ready", true), ("fault", false) },
            readings[1].Bits.Select(b => (b.Name, b.Value)));
    }

    [Fact]
    public void BuildingOne()
    {
        const string ch =
            "number,bytes,type,kind,derived,default,name\n"
            + "1,2,UI,const,,0x7E7E,sync\n"
            + "2,2,UI,plain,,,speed\n"
            + "3,2,UI,derived,crc16/x25 1..2,,crc\n";

        using var defs = Definitions.Parse(ch);
        var frame = defs.Encode([Value.Physical(2, 120)], out var issues);

        Assert.Empty(issues);
        Assert.Equal(new byte[] { 0x7E, 0x7E }, frame[..2]);
        Assert.Equal(new byte[] { 0, 0 }, frame[4..]);

        Assert.Empty(defs.Seal(frame));
        Assert.NotEqual(new byte[] { 0, 0 }, frame[4..]);
        Assert.Empty(defs.DerivedMismatches(frame));
    }

    [Fact]
    public void HeadersInAnotherLanguage()
    {
        using var german = ColumnVocabulary.Create()
            .Ch("Nummer", ChColumn.Number)
            .Ch("Bytes", ChColumn.Bytes)
            .Ch("Bezeichnung", ChColumn.Name);

        using var defs = Definitions.Parse("Nummer,Bytes,Bezeichnung\n7,4,Frame\n", null, german);

        Assert.Empty(defs.Issues);
        Assert.Equal(7u, defs.Channels[0].Number);
        Assert.Equal("Frame", defs.Channels[0].Name);
    }

    [Fact]
    public void EditingADefinitionFile()
    {
        using var grid = Grid.Parse("number,bytes,memo\r\n1,2,first\r\n");

        Assert.Equal(new[] { "number", "bytes", "memo" }, grid.Header);
        Assert.Equal("first", grid.Cell(0, 2));

        grid.SetCell(0, 1, "4");
        Assert.Equal("number,bytes,memo\r\n1,4,first\r\n", grid.ToCsv());
    }
}
