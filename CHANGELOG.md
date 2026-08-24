# Changelog

All notable changes to **chdef** are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The 0.0.x line treats each `0.0.x → 0.0.(x+1)` bump as MAJOR-equivalent
(Cargo's pre-1.0 convention): breaking changes are allowed within 0.0.x and
announced under `### Breaking`. The trunk is `main`; a release is a `vX.Y.Z`
tag, published to crates.io.

## [Unreleased]

### Added
- `ChannelDef::displayed_value` / `render`: which reading the `format`
  column selects, and a default text form of it a consumer may replace
  (ADR-0015). They take a raw integer, so no byte order is assumed.
- `ChannelLayout::capacity` / `with_capacity`: the layout carries the
  capacity it is measured against, and `check_capacity()` reads it
  (ADR-0016). `Definitions::of` puts it in the JSON without restating.
- `Decoded::bits`: the named bits of a decoded channel and whether each is
  set, so a consumer displaying a bit field writes no shifts.
- `ChTable` / `BfTable`: `header`, `rows` and `row(index)` hand over the
  grid an editor draws.
- `CsvStyle` / `LineEnding`, with `style` / `set_style`: a table writes the
  byte-order mark and record separator it read, so editing one cell of a
  file kept with `
` endings no longer rewrites every line (ADR-0017). A
  file that already follows the write rules round-trips byte for byte; a
  table created in code still writes a BOM and `
`.

### Changed
- `DisplayFormat` is `ValueDisplay { Physical, Raw }`, naming the choice
  the `format` column makes rather than the base its cell is spelled in
  (ADR-0015). `parse` still reads `DEC` / `HEX` and `as_str` writes them.
- Issue `hex_with_lsb` is `raw_display_with_lsb`, and the definitions JSON
  spells `format` as `"physical"` / `"raw"`.
- `check_capacity` takes no argument and reads the layout's capacity.

### Removed
- `ChannelDef::format_value`, which rendered with the byte order hardcoded
  to little-endian. `render` replaces it.

## [0.0.2] - 2026-08-24

### Added
- `chdef` crate: `parse_ch_csv` / `parse_bf_csv` / `load_ch_csv` /
  `load_bf_csv` / `build_layout` / `ChannelDef::raw_to_value_endian`, moved
  from `channel.rs` / `csv_loader.rs` of `chbridge-core` without changing
  behaviour. `Endian` and `ChdefError` are the crate's own types.
- Specification (`docs/spec/`) and ADR-0001 / ADR-0002 (`docs/decisions/`).
- `ChannelDef::value_to_raw` / `value_to_bytes` / `value_to_bytes_endian`:
  physical → raw (half away from zero, clamped to the channel width, two's
  complement, 1–8 bytes) and `ChannelDef::bits`.
- `ChColumn` / `BfColumn` / `ColumnMap` / `HeaderLanguage`: columns are
  identified by header name in English (`number,bytes,…`) or Japanese
  (`番号,バイト数,…`), case-insensitively; a first row without a `number`
  column is data in canonical order (ADR-0003).
- `parse_ch_csv_bytes` / `parse_bf_csv_bytes`: parse a CH / BF CSV from bytes
  for a consumer that holds no path (a file dialog returning a stream, a
  browser file input). They drop leading BOMs and decode as UTF-8, failing
  with `ChdefError::Encoding { valid_up_to }` on anything else (ADR-0004).
- `Issue` / `IssueCode` / `Parsed`: a problem in one row no longer stops
  loading — every readable row is read and the problem comes back as an
  `Issue { code, row, col, message }` next to the value
  (`docs/spec/diagnostics.md`). 16 of the 19 specified codes are emitted;
  `bf_bit_out_of_range`, `bf_parent_not_bitfield` and
  `layout_exceeds_capacity` need cross-file input and are still open.
- Blank rows and rows whose first cell starts with `#` are skipped without
  an Issue.
- `ChannelDef::new` / `BitFieldDef::new`: construct a definition with its
  identity; every other field starts at its unspecified value and is set
  directly (ADR-0005).
- `serde` became an opt-in feature; the default build no longer depends on
  it (ADR-0005).
- `min` / `max` are interpreted: physical bounds, or raw bit patterns with
  the `0x` prefix (`Bound`), carried on `ChannelDef` and never applied by a
  conversion; `min_value` / `max_value` / `range_contains` /
  `clamp_to_range` are the explicit queries, and `min_invalid` /
  `max_invalid` / `min_max_swapped` the new Issues (ADR-0006).
- `ChannelLayout::endian`: the whole-layout byte order of `layout.md` §2,
  set by the consumer (`Little` when unset); frame encode / decode will
  consume it.
- `raw_to_bytes_endian` / `raw_from_bytes_endian`: the storey below the
  physical conversion — a raw bit pattern to / from the channel's bytes,
  truncating to the width with no rounding and no clamp (ADR-0007).
- `ChannelLayout::decode` / `channel_bytes` (`conversion.md` §6): slice a
  frame into per-channel bytes with raw and physical readings under the
  layout's `endian`; a channel that overruns a short frame is omitted.
- Golden vector sets `widths`, `scaling`, `bitfields` and `diagnostics`,
  and the `B` (byte order), `F` (BF bit values) and `P` (expected Issues)
  lines the last three needed. The contract now covers all eight legal
  widths, both byte orders, non-zero `lsb` / `offset`, the BF default
  merge, and the Issues a broken definition set produces — the areas the
  first set was blind to, one of which hid the width defect above.
- Golden vectors (`crates/chdef/vectors/`, `interchange.md` §3, ADR-0013):
  the cross-language contract as `ch.csv` / `bf.csv` / `vectors.txt` per set,
  shipped inside the package, with a harness that runs every set and names
  the vector file and line on a mismatch.
- `interchange` module behind the `serde` feature (`interchange.md` §1 / §2,
  ADR-0013): `Definitions::of(&layout, &issues)` (with `with_capacity`),
  `Readings::of(&decoded)` and `ChTable::to_json` / `BfTable::to_json` build
  the documented JSON shapes as their own types, separate from the domain
  types so the wire format and the definitions can grow independently. The
  consumer picks the serializer; chdef depends on none.
- `Value` implements `Display`, writing exactly what `Value::parse` reads.
- `ChannelDef` carries `section` / `memo` / `var` / `format` / `favorite`
  and `BitFieldDef` carries `memo`, so a consumer no longer re-scans the
  cells to recover columns chdef already read. `DisplayFormat` (`DEC` /
  `HEX`) is a type with its own parse; it never affects a conversion.
- `ChannelDef::raw_to_value_u64`: the physical conversion for a raw value
  already held as an integer, without the byte round trip.
- `ChannelLayout::positions`: every channel with its byte offset, in row
  order — the walk `encode` and `decode` perform.
- `BfTable::cross_issues` (ADR-0012): the layout's cross-file BF checks,
  run on the grid where rows still exist — each finding carries the row
  and the `number` / `bit` column, so an editor can point at the cell.
- `ChannelLayout::encode` / `channel_default` (`conversion.md` §5 / §4,
  ADR-0011): build a frame from per-channel `Value`s — physical converted
  and clamped, raw truncated, unnamed channels filled with their default
  with BF bits folded in. Unknown numbers and non-finite values are
  reported (`encode_unknown_channel` / `encode_value_invalid`), never
  dropped. `Value::parse` reads the `0x`-raw / plain-physical notation for
  consumer input.
- `ChTable` / `BfTable` (`docs/spec/editing.md`, ADR-0009): the Table stage
  as a verbatim cell grid — unknown columns, comment rows and header
  spelling survive read → edit → `to_csv` at cell granularity. Cell / row
  edits, `insert_channel` (typed insertion into the columns the file has),
  and `insert_channel_renumbering`, which shifts later numbers with their
  BF parents and reports every `(old, new)` pair as `Renumbered`
  (ADR-0010). `parse_ch_csv` / `parse_bf_csv` are reimplemented on the
  Table with unchanged behaviour.
- `build_layout` runs the cross-file BF checks and returns
  `Parsed<ChannelLayout>` (`bf_parent_not_bitfield` / `bf_bit_out_of_range`,
  without rows — ADR-0008); `ChannelLayout::check_capacity` reports
  `layout_exceeds_capacity`. Every code of the diagnostics spec is now
  emitted. `BitFieldDef::bit_of` extracts a bit from the parent's raw value.

### Fixed
- An unterminated quote read the rest of the file into one cell, so the
  later rows vanished with no Issue and no error — the exact loss
  `format.md` §1 warns about, and the error `diagnostics.md` §1 promises.
  `ChdefError::CsvParse` was unreachable; every entry point now refuses
  such a file and names the line the quote opened on. Its `row` field
  became `line`, saying which base it counts in.
- `default` was capped at 32 bits, so a channel wider than 4 bytes could
  not state one and got `default_invalid` — an Issue whose message denied
  that a well-formed `0x` value was one. A default is now as wide as its
  channel, and a value past that width is `raw_out_of_range` with the low
  bits kept, in decimal as well as hexadecimal, matching the verdict the
  same text already got in `min` / `max`.
- A BF `bit` of 64 or more was reported as `bf_bit_invalid` ("not an
  integer") instead of `bf_bit_out_of_range`.
- `insert_channel` left `favorite` empty instead of writing `0`.
- `encode` truncated a raw value past the channel width without reporting
  it.
- `remove_row` panicked on an index past the end; it returns `None`.
- `Renumbered.moved` repeated a pair once per row rather than once per
  channel, and renumbering a channel at `u32::MAX` overflowed.
- Conversions used two different widths: `raw_to_value_endian` measured a
  channel by its `DataType` while everything else measured it by
  `byte_count`, so a 3-, 5-, 6-, 7- or 8-byte channel read only its first
  two bytes. A 3-byte `SI` channel round-tripped `−100 000` as `+31 072`,
  and `ChannelLayout::decode` returned a `raw` and a `value` that
  disagreed inside one `Decoded` (ADR-0014).
- `raw_to_value_u64` did not sign-extend a 64-bit `SI` channel, so `−1`
  read back as `1.8e19` and every range query built on `min_value` /
  `max_value` inherited it.
- `value_to_raw` clamped in f64, where `2^n − 1` is not representable
  beyond 53 bits; a 7-byte unsigned channel clamped to `0` instead of its
  maximum.
- A `byte_count` of 0 made `bits()` zero and overflowed a subtraction in
  `raw_to_value_u64`.
- `BF` channels read big-endian were shifted, because the byte reader
  zero-padded at the tail regardless of byte order.

### Changed
- `DataType` is `UI` / `SI` / `BF` — the interpretation only (ADR-0014,
  `layout.md` §6). `byte_count()` and `resolve()` are gone; `as_str()` and
  `Display` give the two-letter tag. `ChannelDef::width()` is the single
  authority for how wide a channel is, holding `byte_count` to 1–8.
- `load_ch_csv` / `load_bf_csv` take `impl AsRef<Path>` instead of `&str`, so
  a `PathBuf` and a path that is not valid Unicode both go through. Calls
  passing a string literal are unaffected.
- `parse_ch_csv` / `parse_bf_csv` / their `_bytes` and `load_` forms return
  `Parsed<Vec<…>>` (value plus Issues) instead of a bare `Vec`, and keep
  duplicate rows; `build_layout` now drops duplicates first-wins.
- `BitFieldDef::default_value` is `Option<u8>`: an empty or invalid BF
  `default` is unspecified (the parent channel's bit is kept), no longer 0.
- `ChannelDef::lsb` is stored resolved: an empty, `0`, or invalid `lsb`
  arrives as `1.0` instead of `0.0`.
- `Bound` is renamed `Value` (ADR-0011): the same notation-carrying pair
  now feeds `min` / `max`, form input and encode. `ChannelDef::min` /
  `max` semantics are unchanged.
- `ChannelLayout::total_bytes` is a method computed on demand instead of a
  stored field, so an edited `byte_count` can no longer leave it stale
  (ADR-0006).
- `ChdefError` is `#[non_exhaustive]` like the other growing vocabularies
  (ADR-0005): external matches need a catch-all arm.
- The public surface is the crate root only (ADR-0005): the `channel` /
  `columns` / `csv` / `error` / `issue` module paths are private, and
  `ChColumn` / `BfColumn` / `ColumnMap` / `HeaderLanguage` are withdrawn
  until the writer exists. `ChannelDef`, `BitFieldDef`, `ChannelLayout` and
  `DataType` are `#[non_exhaustive]`, so the fields and variants the
  specification already promises can arrive without breaking callers.
- `parse_ch_csv` / `parse_bf_csv` locate every column by header name instead
  of position; the default column matches exact spellings (`default`,
  `値(デフォルト)`, `デフォルト値`, `DefaultValue`) rather than any header
  containing "デフォルト".
