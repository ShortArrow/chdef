# chdef

🌐 **English** | [日本語](docs/README.jp.md)

[![CI](https://github.com/ShortArrow/chdef/actions/workflows/ci.yml/badge.svg)](https://github.com/ShortArrow/chdef/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Channel definitions (CH) and bit-field definitions (BF) for binary
> frames: parse the CSVs, compute the layout, convert raw ↔ physical.

> **⚠ Pre-alpha (0.0.x).** The API and the CSV rules are still moving;
> until 0.1.0 lands, patch releases may break either.

---

## What it is

A **CH definition** (channel definition CSV) gives every field of a binary
frame a meaning; a **BF definition** (bit-field definition CSV) gives every
bit of a `BF`-typed channel a name. `chdef` is the one Rust implementation
of both, so that every consumer reads the same file the same way.

- Parse a CH CSV / BF CSV — a column has one canonical name
  (`number,bytes,…`) and every other spelling a header may use is a
  **vocabulary you supply**, so a header in any language is read by
  teaching its spellings; a leading BOM is ignored and rows without an
  integer `number` are skipped
- Compute the frame layout (each channel sits at the cumulative `bytes`,
  plus the total byte count)
- Convert raw ↔ physical: `value = raw × lsb + offset` and back
  (`lsb` 0 / empty is 1, `SI` is signed, endianness selectable, rounding half
  away from zero, clamped to the channel width)

```rust
// The canonical names, with no vocabulary at all.
let channels = chdef::parse_ch_csv(ch_csv_text)?;
let bitfields = chdef::parse_bf_csv(bf_csv_text)?;
let layout = chdef::build_layout(channels, bitfields);
let value = layout.channels[0].raw_to_value(&frame[..4]);
let bytes = layout.channels[0].value_to_bytes(value);
```

A header in another vocabulary — one chdef ships, or one you build — is
read the same way:

```rust
let japanese = chdef::ColumnVocabulary::japanese();
let german = chdef::ColumnVocabulary::new()
    .ch("Nummer", chdef::ChColumn::Number)
    .ch("Bytes", chdef::ChColumn::Bytes);

let channels = chdef::parse_ch_csv_with(ch_csv_text, &japanese)?;
```

The columns of both CSVs, their canonical names, and how each cell is read
are specified in [docs/spec/format.md](./docs/spec/format.md).

The specification lives in [docs/spec/](./docs/spec/README.md); design
decisions in [docs/decisions/](./docs/decisions/README.md).

## From C# and C

The same implementation, reached through a C ABI. What crosses it is every
rule the specification states, so a consumer in another language never
writes one of them a second time
([docs/spec/abi.md](./docs/spec/abi.md)).

```sh
dotnet add package Chdef
```

The NuGet package carries the native library for `linux-x64`, `win-x64`,
`osx-arm64` and `osx-x64`, so nothing else is needed.

```csharp
using var defs = Definitions.Parse(chCsv, bfCsv);
var frame = defs.Encode([Value.Parse("0x0004", 1)], out var issues);
foreach (var reading in defs.Decode(frame))
{
    foreach (var bit in reading.Bits) { /* name and value of each bit */ }
}

using var grid = Grid.Parse(File.ReadAllBytes(path));
grid.SetCell(0, 1, "4");
File.WriteAllBytes(path, grid.ToCsvBytes());
```

For C, the header is
[crates/chdef-capi/include/chdef.h](./crates/chdef-capi/include/chdef.h)
and the library is the `chdef-capi` crate built as a `cdylib`.

## Origin

The CH / BF concept was extracted from `chbridge-core` of chbridge, an
internal telemetry bridge. Definition files
themselves (real-device channel tables) belong to each consumer; this
repository holds synthetic data only.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
