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

**Nothing here throws for a bad row.** A problem comes back as an `Issue`
beside the value it is about, so a file that is wrong in one cell still
loads. Only an unreadable *file* — bad encoding, an unterminated quote —
is an `Err`.

## Reading a frame

```rust
let ch_csv = "number,bytes,type,name,lsb,offset,unit\n\
              1,2,UI,speed,0.5,0,km/h\n\
              2,1,BF,status,1,0,\n";
let bf_csv = "number,bit,name\n\
              2,0,ready\n\
              2,1,fault\n";

let channels = chdef::parse_ch_csv(ch_csv).expect("readable CH CSV");
let bitfields = chdef::parse_bf_csv(bf_csv).expect("readable BF CSV");
let layout = chdef::build_layout(channels.value, bitfields.value).value;

// 0x0040 little-endian is raw 64, which is 32 km/h at lsb 0.5.
let readings = layout.decode(&[0x40, 0x00, 0b01]);

assert_eq!(readings[0].channel.name, "speed");
assert_eq!(readings[0].value, 32.0);

let bits: Vec<(&str, bool)> = readings[1]
    .bits()
    .map(|(bit, set)| (bit.name.as_str(), set))
    .collect();
assert_eq!(bits, vec![("ready", true), ("fault", false)]);
```

## Building one

`encode` writes the values you give and the defaults of the channels you
do not. A channel the definitions mark as `derived` — a CRC — is filled by
`seal`, which is a call of its own so that `encode` stays a pure function
of what you handed it.

```rust
let ch_csv = "number,bytes,type,kind,derived,default,name\n\
              1,2,UI,const,,0x7E7E,sync\n\
              2,2,UI,plain,,,speed\n\
              3,2,UI,derived,crc16/x25 1..2,,crc\n";

let read = chdef::parse_ch_csv(ch_csv).expect("readable CH CSV");
let layout = chdef::build_layout(read.value, Vec::new()).value;

let encoded = layout.encode(&[(2, chdef::Value::Physical(120.0))]);
assert!(encoded.issues.is_empty());

let mut frame = encoded.value;
assert_eq!(&frame[..2], &[0x7E, 0x7E], "the const channel took its default");
assert_eq!(&frame[4..], &[0, 0], "encode does not compute the CRC");

let unsealed = layout.seal(&mut frame);
assert!(unsealed.is_empty());
assert_ne!(&frame[4..], &[0, 0], "seal did");

// The receiver checks the same way round.
assert!(layout.derived_mismatches(&frame).is_empty());
```

## Headers in another language

A header spelled any other way is read by teaching its spellings. The
vocabulary chdef ships for the Japanese column names is one such value and
has no standing one you build lacks.

```rust
use chdef::{ChColumn, ColumnVocabulary};

let german = ColumnVocabulary::new()
    .ch("Nummer", ChColumn::Number)
    .ch("Bytes", ChColumn::Bytes)
    .ch("Bezeichnung", ChColumn::Name);

let read = chdef::parse_ch_csv_with("Nummer,Bytes,Bezeichnung\n7,4,Frame\n", &german)
    .expect("readable CH CSV");

assert!(read.issues.is_empty());
assert_eq!(read.value[0].number, 7);
assert_eq!(read.value[0].name, "Frame");

// The same call reads a Japanese header with the shipped vocabulary.
let japanese = ColumnVocabulary::japanese();
let read = chdef::parse_ch_csv_with("番号,バイト数\n7,4\n", &japanese)
    .expect("readable CH CSV");
assert_eq!(read.value[0].number, 7);
```

## Where to look next

| | |
|---|---|
| [docs/guide.md](./docs/guide.md) | The shortest path through each task |
| [docs/spec/](./docs/spec/README.md) | What the format is, exactly |
| [docs/migration.md](./docs/migration.md) | What changed between 0.0.x releases |
| [docs/decisions/](./docs/decisions/README.md) | Why it is the way it is |

## From C# and C

The same implementation, reached through a C ABI. It carries every
rule the specification states, so a consumer in another language never
writes one of them a second time
([docs/spec/abi.md](./docs/spec/abi.md)).

```sh
dotnet add package Chdef
```

The NuGet package carries the native library for `linux-x64`, `win-x64`,
`osx-arm64` and `osx-x64`, so nothing else is needed; its own readme is
[bindings/dotnet/Chdef/README.md](./bindings/dotnet/Chdef/README.md).

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
