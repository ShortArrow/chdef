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
