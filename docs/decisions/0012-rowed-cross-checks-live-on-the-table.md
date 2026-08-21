# ADR-0012: Rowed cross-file checks live on the Table; the layout's stay rowless

- Status: Accepted
- Date: 2026-08-21
- Release: 0.0.2

## Context

ADR-0008 put the cross-file BF checks into `build_layout` and accepted
`row: None`, reasoning that "the only place both sides meet is
`build_layout`" and that by then the CSV rows are gone. Both facts have
since changed by half. The Table stage (ADR-0009) now holds both files
with their grid rows intact, so a second meeting place exists. And a
consumer (sensord) reported re-implementing exactly these checks with
row numbers because a grid editor cannot paint a cell red from a
rowless Issue.

## Decision

- **`BfTable::cross_issues(&[ChannelDef])`** runs the same two checks —
  parent missing or not `BF`, bit at or beyond the parent width — on
  the grid, where each finding carries its row and its `number` / `bit`
  column. Rows whose `number` or `bit` does not parse are not repeated
  here; `bitfields()` already reports them with rows.
- **`build_layout` keeps its rowless issues** (ADR-0008 stands): a
  layout is built from typed rows that may never have come from a file,
  and `BitFieldDef` still carries no file coordinates.
- The same code (`bf_parent_not_bitfield` / `bf_bit_out_of_range`) is
  emitted from both places; the consumer picks the reporting surface
  that matches what it holds — a grid, or bare definitions.

## Alternatives rejected

- **Source rows on `BitFieldDef` / `ChannelDef`** (the consumer's
  suggestion): ADR-0008's reasoning holds — file coordinates inside a
  domain type tax every constructor and consumer for one diagnostic
  path, and the Table now provides that path without the tax.
- **Removing the checks from `build_layout`**: a consumer without
  tables (defs built in code, or straight through `parse_*`) would lose
  the checks entirely.

## Consequences

- A grid editor calls `bf_table.cross_issues(&ch_table.channels().value)`
  and every finding lands on a cell; sensord's re-implementation
  (`parse_bf_diag`) can be deleted.
- One finding appears twice when a consumer runs both surfaces on the
  same data; deduplication by `(code, number, bit)` is the consumer's,
  matching the diagnostics principle that chdef does not aggregate.
