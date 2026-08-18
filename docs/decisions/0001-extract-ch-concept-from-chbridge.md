# ADR-0001: Extract the CH / BF concept of chbridge into a standalone crate

- **Status**: Accepted
- **Date**: 2026-08-18
- **Release**: 0.0.1

## Context

The CH definition CSV (`番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位,…`)
and the BF definition CSV (`番号,BIT番号,メッセージ名称,値(デフォルト),備考`) are a
format defined by [chbridge](https://github.com/ShortArrow/chbridge). Since
then several consumers (Rust / C# / TypeScript) have come to read the same,
or a derived, column set. The parser, the layout computation and the
raw ↔ physical conversion were reimplemented per language and per consumer,
and the details drifted apart.

## Decision

Move `channel.rs` (`DataType` / `ChannelDef` / `BitFieldDef` /
`ChannelLayout` / `raw_to_value_endian`) and `csv_loader.rs`
(`load_ch_csv` / `load_bf_csv`) of `chbridge-core` into a standalone crate
`chdef` (`MIT OR Apache-2.0`) without changing behaviour. chbridge becomes a
consumer of this crate.

- Definition files themselves (real-device channel tables) stay with each
  consumer; this repository holds synthetic data only.
- Requirements that consumers added later (row / column diagnostics, JSON
  output, encode / decode, golden vectors) are written as the chdef
  specification and implemented in chdef.

## Alternatives considered

- **Keep an implementation in every consumer and share only golden
  vectors**: divergence becomes detectable but does not shrink. Rejected.
- **Use one of the consumers' implementations as the base**: some are ahead
  in features, but keeping the origin of a public crate in chbridge, which
  is already published under a public licence, is simpler. Rejected.

## Consequences

- `chdef` 0.0.1: `parse_ch_csv` / `parse_bf_csv` / `load_ch_csv` /
  `load_bf_csv` / `build_layout` / `ChannelDef::raw_to_value_endian`. The
  chbridge unit tests pass unchanged.
- Subsequent changes (diagnostics, JSON, codec, importers for other formats,
  C ABI) get their own ADRs.
