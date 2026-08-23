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

### Changed
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
