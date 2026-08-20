# ADR-0005: The public surface is the crate root, and data types tolerate the growth the specification already promises

- Status: Accepted
- Date: 2026-08-20
- Release: 0.0.2

## Context

The first crates.io publish freezes everything a caller can observe:
every public path, every constructible struct, every exhaustively
matchable enum. With enough callers, someone depends on all of it, so
whatever ships in the first publish must either stay or break someone.

Three parts of the 0.0.1 surface were promises we already knew we could
not keep:

- The specification names fields that `ChannelDef` does not carry yet
  (`section`, `min`, `max`, `format` — `docs/spec/format.md` §3), an
  `endian` on the layout (`docs/spec/layout.md` §2), and 64-bit width
  suffixes that `DataType` cannot represent. Every one of them was a
  planned breaking change, because the structs were constructible and
  the enum matchable outside the crate.
- Every item was reachable twice (`chdef::parse_ch_csv` and
  `chdef::csv::parse_ch_csv`), and the column vocabulary (`ChColumn`,
  `BfColumn`, `ColumnMap`, `HeaderLanguage`, the fixed-length
  `CANONICAL` arrays) was public although no consumer exists until the
  writer does. A symbol public without a consumer is debt taken on for
  no benefit.
- `serde` was a mandatory dependency used by one derive on `Endian`,
  needed only by the unimplemented interchange. A dependency is part of
  the API: its cadence and licence transfer to every caller.

## Decision

- **The public surface is the re-export list in `lib.rs`.** All modules
  are private; each item has exactly one public path. The column
  vocabulary is withdrawn until the writer gives it a consumer, and
  `DataType::parse` / `resolve` / `category` are crate-internal.
- **`ChannelDef`, `BitFieldDef`, `ChannelLayout` and `DataType` are
  `#[non_exhaustive]`.** Fields stay `pub` — reading and in-place
  editing remain part of the contract — but literal construction is
  reserved to chdef, so the specced fields and variants can arrive
  without breaking callers. Callers construct through `ChannelDef::new`
  / `BitFieldDef::new` (identity arguments, everything else starting at
  its unspecified value) and `build_layout`.
- **`serde` is an opt-in feature** (`features = ["serde"]`); the default
  build does not compile it.

## Alternatives rejected

- **A builder for `ChannelDef`**: ceremony without benefit while every
  field is `pub` and settable directly; `new` plus field writes is the
  same expressiveness in less surface.
- **Keeping the column vocabulary public for the future writer**: the
  writer's needs will shape that API (spelling choice, write-back
  order); freezing today's guess would constrain it. The `CANONICAL`
  arrays also froze their length into their type.
- **`serde` always on**: pays a dependency tax on every consumer for a
  format none of them can use yet.

## Consequences

- Adding `section` to `ChannelDef`, `endian` to `ChannelLayout`, or a
  64-bit variant to `DataType` is a minor change after this ADR; before
  it, each was major.
- External `match` on `DataType` needs a catch-all arm.
- Dead code became visible the moment the surface shrank:
  `DataType::parse` (superseded by the parser's own type reading) and
  `HeaderLanguage` / `ChColumn::name` / `BfColumn::name` (no consumer
  until the writer) were deleted rather than kept as debt. The writer
  reintroduces what it needs when it exists.
- A `compile_fail` doctest on `ChannelDef` pins the external view: the
  test fails if literal construction ever becomes possible again.
- CI tests both feature sets (default and `--all-features`).
