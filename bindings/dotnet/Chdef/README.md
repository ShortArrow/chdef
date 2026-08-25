# Chdef

Channel definitions (CH) and bit-field definitions (BF) for binary frames:
read the CSVs, compute the frame layout, encode and decode frames.

A binding over the [chdef](https://github.com/ShortArrow/chdef) Rust
crate, carrying its native library for `linux-x64`, `win-x64`, `osx-arm64`
and `osx-x64` — `dotnet add package Chdef` and nothing else.

> **⚠ Pre-alpha (0.0.x).** The API and the CSV rules are still moving;
> until 0.1.0 lands, patch releases may break either.

**Nothing here throws for a bad row.** A problem comes back as an `Issue`
beside the value it is about, so a file that is wrong in one cell still
loads. Only an unreadable *file* — bad encoding, an unterminated quote —
raises `ChdefException`.

Every example below is a test in `Chdef.Tests/ReadmeTests.cs`, and the
build fails if this page and that file disagree.

## Reading a frame

```csharp
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
```

## Building one

`Encode` writes the values you give and the defaults of the channels you
do not. A channel the definitions mark as `derived` — a CRC — is filled by
`Seal`, which is a call of its own so that `Encode` stays a pure function
of what you handed it.

```csharp
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
```

## Headers in another language

A column has one canonical name, and every other spelling a header may use
is a vocabulary you supply. The one shipped for the Japanese column names
has no standing one you build lacks.

```csharp
using var german = ColumnVocabulary.Create()
    .Ch("Nummer", ChColumn.Number)
    .Ch("Bytes", ChColumn.Bytes)
    .Ch("Bezeichnung", ChColumn.Name);

using var defs = Definitions.Parse("Nummer,Bytes,Bezeichnung\n7,4,Frame\n", null, german);

Assert.Empty(defs.Issues);
Assert.Equal(7u, defs.Channels[0].Number);
Assert.Equal("Frame", defs.Channels[0].Name);
```

## Editing a definition file

`Grid` is the file as its cells — comment rows, blank rows and unknown
columns included — and it writes back in the shape it was read, byte for
byte when the file already follows the write rules.

```csharp
using var grid = Grid.Parse("number,bytes,memo\r\n1,2,first\r\n");

Assert.Equal(new[] { "number", "bytes", "memo" }, grid.Header);
Assert.Equal("first", grid.Cell(0, 2));

grid.SetCell(0, 1, "4");
Assert.Equal("number,bytes,memo\r\n1,4,first\r\n", grid.ToCsv());
```

## Where to look next

- [The guide](https://github.com/ShortArrow/chdef/blob/main/docs/guide.md)
  — the shortest path through each task
- [The specification](https://github.com/ShortArrow/chdef/blob/main/docs/spec/README.md)
  — what the format is, exactly
- [What crosses the ABI](https://github.com/ShortArrow/chdef/blob/main/docs/spec/abi.md)
  — and the conventions it crosses by
- [Migration](https://github.com/ShortArrow/chdef/blob/main/docs/migration.md)
  — what changed between 0.0.x releases

## License

MIT OR Apache-2.0.
