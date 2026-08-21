# ADR-0008: build_layout returns Issues, and its Issues carry no row

- Status: Accepted
- Date: 2026-08-21
- Release: 0.0.2

## Context

Three diagnostics could not be emitted by the parsers because they need
the CH and BF files together: `bf_parent_not_bitfield`,
`bf_bit_out_of_range` (a BF row against its parent channel) and
`layout_exceeds_capacity` (the whole frame against a consumer-supplied
capacity). The only place both sides meet is `build_layout`. But by that
stage the CSV rows are gone — `BitFieldDef` deliberately carries no
source row, and the diagnostics specification had promised a row for the
two BF codes.

## Decision

- **`build_layout` returns `Parsed<ChannelLayout>`** and performs the
  cross-file checks: a BF row whose parent is missing or not `BF`, or
  whose bit is at or beyond the parent width, is skipped with an Issue.
  Duplicate dropping stays silent here — the parser already reported it.
- **These Issues carry `row: None`.** The specification's row column
  changes to "—" for both codes: an Issue names the `(number, bit)` pair
  in its message, which identifies the row to a human, and attaching
  source rows to `BitFieldDef` would pollute the domain type with file
  coordinates for the sake of one diagnostic.
- **`capacity` is an opt-in query, not a parameter**:
  `ChannelLayout::check_capacity(capacity)` returns the
  `layout_exceeds_capacity` Issue or `None`. Without a capacity there is
  no check, exactly as `docs/spec/layout.md` §5 words it.

## Alternatives rejected

- **A `row` field on `BitFieldDef`**: file coordinates inside a domain
  type; every constructor and consumer pays for one diagnostic's
  convenience.
- **Cross-checking inside `parse_bf_csv` with an optional `&[ChannelDef]`
  argument**: an optional argument that changes which Issues can appear
  makes the same call honest in one call site and silently incomplete in
  another.
- **A `capacity: Option<usize>` parameter on `build_layout`**: a `None`
  most callers pass forever, to serve the few that have a capacity.

## Consequences

- Breaking for `build_layout` callers (`.value`), accepted pre-publish.
- The Issue vocabulary of `docs/spec/diagnostics.md` is fully emitted:
  22 codes of 22.
- A consumer that wants row-accurate BF cross-diagnostics can correlate
  `(number, bit)` against its own grid — the pair is unique per file by
  the `bf_duplicate` rule.
