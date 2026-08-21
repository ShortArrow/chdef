# ADR-0009: The Table is the editing substrate, and the round trip is cell-level

- Status: Accepted
- Date: 2026-08-21
- Release: 0.0.2

## Context

Consumers want to edit definitions — add rows, insert a channel between
two others — and save the file back. `ChannelDef` cannot be that
substrate: it drops unknown columns, the header spelling, comment rows
and cell notation, so a write from it is lossy by construction. The
specification already reserved a Table stage (`docs/spec/layout.md` §1)
holding the file as a verbatim cell grid; it was unimplemented, and the
parsers went straight from text to typed rows.

A second question is what "faithful write-back" promises: byte-identical
output, or something weaker?

## Decision

- **`ChTable` / `BfTable` hold the file as verbatim cells** (header row,
  data rows, comments, blanks, unknown columns) and are what an editing
  session mutates. Rows and Layout stay derived views — interpret again
  after an edit, the same never-stale principle as `total_bytes`. Undo is
  the consumer's (`Clone` is the snapshot).
- **`parse_ch_csv` / `parse_bf_csv` are reimplemented on the Table**
  (`ChTable::parse(text)?.channels()`), so there is one interpretation
  path, not two. Their signatures and behaviour do not change.
- **The round-trip guarantee is cell-level, not byte-level**: rows and
  cell contents survive; quoting, record separators and the BOM are
  normalised to the write rules of `docs/spec/format.md` §1. A fully
  empty line yields no record and is not preserved.
- A **new** table writes the English canonical header
  (`docs/spec/format.md` §2's rule for newly created files).

## Alternatives rejected

- **Byte-identical round trip**: requires remembering the original
  quoting, separator and BOM count per cell and file. The only gain is a
  quieter diff for files no spreadsheet has touched — Excel normalises
  all of it anyway on first save.
- **Editing typed `ChannelDef`s and serialising them**: loses everything
  the type does not carry; acceptable for creating new files (that path
  exists as `insert_channel` on a new table) but not for editing files
  we did not write.
- **Caching the interpreted rows inside the table**: reintroduces the
  stale-derived-state class the crate just removed.

## Consequences

- `docs/spec/editing.md` specifies the operations and the guarantee.
- The writer half of `docs/spec/format.md` §1 is implemented by
  `to_csv`; `csv_loader`-style consumers keep reading through the
  unchanged parse functions.
- The 16 canonical columns' English spellings returned to the crate
  (crate-internal `en()`), as ADR-0005 predicted the writer would do.
