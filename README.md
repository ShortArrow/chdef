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

- Parse a CH CSV / BF CSV — columns are found by header name, spelled in
  English (`number,bytes,…`) or Japanese (`番号,バイト数,…`); a leading BOM is
  ignored and rows without an integer `number` are skipped
- Compute the frame layout (each channel sits at the cumulative `bytes`,
  plus the total byte count)
- Convert raw ↔ physical: `value = raw × lsb + offset` and back
  (`lsb` 0 / empty is 1, `SI` is signed, endianness selectable, rounding half
  away from zero, clamped to the channel width)

The columns of both CSVs, with their aliases and how each cell is read, are
specified in [docs/spec/format.md](./docs/spec/format.md).

```rust
let channels = chdef::parse_ch_csv(ch_csv_text)?;
let bitfields = chdef::parse_bf_csv(bf_csv_text)?;
let layout = chdef::build_layout(channels, bitfields);
let value = layout.channels[0].raw_to_value(&frame[..4]);
let bytes = layout.channels[0].value_to_bytes(value);
```

The specification lives in [docs/spec/](./docs/spec/README.md); design
decisions in [docs/decisions/](./docs/decisions/README.md).

## Origin

The CH / BF concept was extracted from `chbridge-core` of
[chbridge](https://github.com/ShortArrow/chbridge). Definition files
themselves (real-device channel tables) belong to each consumer; this
repository holds synthetic data only.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
