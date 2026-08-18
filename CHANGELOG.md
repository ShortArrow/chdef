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

### Changed
- `parse_ch_csv` / `parse_bf_csv` locate every column by header name instead
  of position; the default column matches exact spellings (`default`,
  `値(デフォルト)`, `デフォルト値`, `DefaultValue`) rather than any header
  containing "デフォルト".
