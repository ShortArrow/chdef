// The four things a consumer said it could not stop reimplementing:
// the bits of a definition and of a reading, the cells of the file, the
// notation of a value, and the text form of a reading (ADR-0023).
//
// The same sentences of docs/spec that crates/chdef-capi/tests/
// spec_abi_surface.rs stands on, asked through the managed API instead.

using System.Text;
using Xunit;

namespace Chdef.Tests;

public sealed class BitsAndGridTests
{
    private const string Ch =
        "number,bytes,type,name,lsb,offset,unit,default,format\n"
        + "1,2,BF,status,1,,,5,\n"
        + "2,1,UI,speed,1,10,km/h,,HEX\n"
        + "3,1,UI,temp,0.5,0,C,,DEC\n";

    private const string Bf =
        "number,bit,name,default,memo\n"
        + "1,0,ready,,keeps\n"
        + "1,2,fault,0,cleared\n"
        + "1,7,alarm,1,set\n";

    private static readonly string[] BitNames = ["ready", "fault", "alarm"];
    private static readonly uint[] BitNumbers = [0, 2, 7];
    private static readonly string[] GridHeader = ["number", "bytes", "memo"];
    private static readonly string[] GridFirstRow = ["1", "2", "first"];

    [Fact]
    public void AChannelCarriesTheBitsItsBfRowsName()
    {
        using var defs = Definitions.Parse(Ch, Bf);
        var status = defs.Channels[0];

        Assert.Equal(3, status.Bits.Count);
        Assert.Equal(BitNumbers, status.Bits.Select(b => b.Number));
        Assert.Equal(BitNames, status.Bits.Select(b => b.Name));
        Assert.Equal("keeps", status.Bits[0].Memo);
        Assert.All(status.Bits, b => Assert.Equal(1u, b.Channel));

        // format.md §3: an empty BF `default` means the bit keeps the parent
        // channel's, which must not collapse into a default of 0.
        Assert.Null(status.Bits[0].Default);
        Assert.Equal(0, status.Bits[1].Default);
        Assert.Equal(1, status.Bits[2].Default);

        Assert.Empty(defs.Channels[1].Bits);
    }

    [Fact]
    public void ADecodedFrameNamesTheValueOfEachBit()
    {
        // conversion.md §4: channel 1 defaults to 5 (bits 0 and 2 set); bit 0
        // names no default so it stays 1, bit 2 is cleared, bit 7 is set.
        using var defs = Definitions.Parse(Ch, Bf);
        var frame = defs.Encode([], out var issues);
        Assert.Empty(issues);
        Assert.Equal(0x81, frame[0]);

        var status = defs.Decode(frame)[0];
        Assert.Equal(3, status.Bits.Count);
        Assert.Equal(
            new[] { ("ready", true), ("fault", false), ("alarm", true) },
            status.Bits
                .Zip(defs.Channels[0].Bits, (reading, def) => (def.Name, reading.Value)));

        Assert.Empty(defs.Decode(frame)[1].Bits);
    }

    [Fact]
    public void TheFormatColumnSelectsWhichReadingIsShown()
    {
        // conversion.md §7: DEC means the physical value, HEX the raw one.
        using var defs = Definitions.Parse(Ch, Bf);

        var hex = defs.Displayed(1, 20);
        Assert.True(hex.IsRaw);
        Assert.Equal(20ul, hex.RawValue);
        Assert.Equal(2u, hex.Channel);

        var dec = defs.Displayed(2, 20);
        Assert.False(dec.IsRaw);
        Assert.Equal(10.0, dec.PhysicalValue);
    }

    [Fact]
    public void ARawReadingRendersInHexadecimalPaddedToTheChannelWidth()
    {
        using var defs = Definitions.Parse(Ch, Bf);
        Assert.Equal("0x14", defs.Render(1, 20));
    }

    [Fact]
    public void ALeadingHexPrefixMeansRawAndAnythingElseMeansPhysical()
    {
        // format.md §3.
        var raw = Value.Parse("0x14", 7);
        Assert.True(raw.IsRaw);
        Assert.Equal(20ul, raw.RawValue);
        Assert.Equal(7u, raw.Channel);

        var physical = Value.Parse("-1.5", 7);
        Assert.False(physical.IsRaw);
        Assert.Equal(-1.5, physical.PhysicalValue);

        Assert.True(Value.Parse("0X14", 7).IsRaw);
    }

    [Fact]
    public void TextThatDenotesNoValueIsReportedRatherThanGuessed()
    {
        Assert.False(Value.TryParse("not a number", 1, out _));
        var thrown = Assert.Throws<ChdefException>(() => Value.Parse("not a number", 1));
        Assert.Equal(-8, thrown.Status);
    }

    [Fact]
    public void AParsedValueGoesStraightBackIntoEncode()
    {
        using var defs = Definitions.Parse(Ch, Bf);
        var frame = defs.Encode([Value.Parse("0x0004", 1)], out var issues);
        Assert.Empty(issues);
        Assert.Equal(0x04, frame[0]);
    }

    // -------------------------------------------------------------- grid

    private const string GridCsv = "﻿number,bytes,memo\r\n1,2,first\r\n# a comment\r\n";

    [Fact]
    public void AGridIsTheHeaderAndEveryRowIncludingComments()
    {
        // editing.md §3.
        using var grid = Grid.Parse(GridCsv);
        Assert.Equal(GridHeader, grid.Header);
        Assert.Equal(2, grid.RowCount);
        Assert.Equal("first", grid.Cell(0, 2));
        Assert.Equal("# a comment", grid.Cell(1, 0));
        Assert.Equal(GridFirstRow, grid.Row(0));
    }

    [Fact]
    public void AFileAlreadyInTheWriteRulesRoundTripsByteForByte()
    {
        // editing.md §2, byte-order mark and record separator included.
        using var grid = Grid.Parse(GridCsv);
        Assert.Equal(GridCsv, grid.ToCsv());
        Assert.Equal(Encoding.UTF8.GetBytes(GridCsv), grid.ToCsvBytes());
    }

    [Fact]
    public void SettingACellPastTheEndOfAShortRowPadsIt()
    {
        using var grid = Grid.Parse(GridCsv);
        grid.SetCell(1, 3, "late");
        Assert.Equal(4, grid.ColumnCount(1));
        Assert.Equal("", grid.Cell(1, 1));
        Assert.Equal("late", grid.Cell(1, 3));
    }

    [Fact]
    public void SettingACellOutsideTheGridThrowsAndAddsNothing()
    {
        using var grid = Grid.Parse(GridCsv);
        var thrown = Assert.Throws<ChdefException>(() => grid.SetCell(9, 0, "stray"));
        Assert.Equal(-2, thrown.Status);
        Assert.Equal(2, grid.RowCount);
    }

    [Fact]
    public void AFileKeptWithoutABomAndWithLfEndingsKeepsThem()
    {
        // editing.md §2: editing one cell of such a file does not rewrite
        // every line of it.
        using var grid = Grid.Parse("number,bytes\n1,2\n");
        Assert.Equal("number,bytes\n1,2\n", grid.ToCsv());

        grid.SetCell(0, 1, "4");
        Assert.Equal("number,bytes\n1,4\n", grid.ToCsv());
    }

    [Fact]
    public void ARowIsInsertedEmptyAndFilledWithCellWrites()
    {
        using var grid = Grid.Parse(GridCsv);
        grid.InsertRow(0);
        Assert.Equal(3, grid.RowCount);
        Assert.Equal(0, grid.ColumnCount(0));

        grid.SetCell(0, 0, "2");
        Assert.Equal("2", grid.Cell(0, 0));
        Assert.Equal("first", grid.Cell(1, 2));
    }

    [Fact]
    public void RemovingARowOutsideTheGridRemovesNothingAndThrows()
    {
        using var grid = Grid.Parse(GridCsv);
        var thrown = Assert.Throws<ChdefException>(() => grid.RemoveRow(9));
        Assert.Equal(-2, thrown.Status);
        Assert.Equal(2, grid.RowCount);

        grid.RemoveRow(0);
        Assert.Equal(1, grid.RowCount);
        Assert.Equal("# a comment", grid.Cell(0, 0));
    }

    [Fact]
    public void AnEditedCellIsWhatComesBackOut()
    {
        using var grid = Grid.Parse(GridCsv);
        grid.SetCell(0, 1, "4");
        Assert.Equal(
            "﻿number,bytes,memo\r\n1,4,first\r\n# a comment\r\n",
            grid.ToCsv());
    }

    [Fact]
    public void ACellHoldingASeparatorIsQuotedOnTheWayOut()
    {
        // format.md §1: a cell is quoted only when it holds `,` `"` or a
        // newline.
        using var grid = Grid.Parse(GridCsv);
        grid.SetCell(0, 2, "a,b");
        Assert.Contains("\"a,b\"", grid.ToCsv());
    }

    [Fact]
    public void AUsedGridRefusesToBeUsedAfterDisposal()
    {
        var grid = Grid.Parse(GridCsv);
        grid.Dispose();
        Assert.Throws<ObjectDisposedException>(() => grid.RowCount);
        grid.Dispose();
    }

    [Fact]
    public void StructurallyBrokenCsvIsAnExceptionRatherThanASilentTruncation()
    {
        var thrown = Assert.Throws<ChdefException>(() => Grid.Parse("a,b\n\"unterminated\n"));
        Assert.NotEmpty(thrown.Message);
    }
}
