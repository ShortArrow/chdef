# ADR-0010: Renumbering shifts both files and reports its effects; outside references are the consumer's

- Status: Accepted
- Date: 2026-08-21
- Release: 0.0.2

## Context

"Insert a channel between ch3 and ch4" means two different things. If
`number` is a pure identity, the new channel takes any free number and
nothing else moves — the specification already allows gaps
(`docs/spec/layout.md` §4). If numbers are to stay consecutive, ch4 and
everything after must shift up by one, `BitFieldDef` parent references
must follow, and every reference **outside** the two CSVs — chbridge's
TOML `const_channels`, notes, code — silently breaks.

chdef cannot repair references it does not know about. The question was
what it owes their owners.

## Decision

- Both insertions exist: plain `insert_channel` (free number, nothing
  moves) and `insert_channel_renumbering` (consecutive numbering).
- The renumbering variant **keeps the repository's own invariant** — BF
  parent numbers shift with their channels — and **returns its effects
  as data**: `Renumbered { moved: Vec<(old, new)> }`. That is the same
  philosophy as Issues (diagnostics as data, not callbacks): the
  consumer decides whether `moved` becomes a UI notice, a TOML rewrite,
  or nothing.
- Outside references are explicitly not chdef's responsibility;
  `docs/spec/editing.md` §4 says so.

## Alternatives rejected

- **Only free-number insertion**: humans editing a numbered CSV expect
  consecutive numbers; refusing the operation pushes an error-prone
  multi-cell edit onto every consumer.
- **Only renumbering insertion**: destroys the identity-stable workflow
  the gap rule exists for.
- **A change-notification callback or event stream**: a stateful
  observer API for a value-based library; the return value carries the
  same information with none of the coupling.

## Consequences

- `Renumbered` is `#[non_exhaustive]`; a later `moved_bits` or similar
  can be added without breaking.
- A consumer that renumbers via raw `set_cell` gets no report and no BF
  follow-up — the invariant lives in the operation, and the raw grid
  stays a raw grid.
