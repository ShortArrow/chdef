# ADR-0020: A grid is the uninterpreted file, and the typed tables hold one

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.3

## Context

A consumer that displays and edits a definition without reading its
columns had to pick `ChTable` or `BfTable` by the suffix in the name, for
an operation — read the cells — that is identical in both. The two types
differ only in their column vocabulary (`ColumnMap<ChColumn>` versus
`ColumnMap<BfColumn>`); everything about cells, rows, the file's shape
and writing it back was the same, and was in fact generated into both
types by a `grid_api!` macro.

So the request and an internal duplication pointed at the same missing
type.

## Decision

- **`Grid` is the file with nothing interpreted**: the header row, every
  data row, the shape it was read in, and the operations that need no
  column vocabulary — `header` / `rows` / `row` / `cell` / `row_count` /
  `set_cell` / `insert_row` / `append_row` / `remove_row` / `style` /
  `set_style` / `to_csv` / `to_json`.
- **`ChTable` and `BfTable` hold a `Grid`** and add only what needs a
  vocabulary: `channels` / `bitfields`, `insert_channel`,
  `insert_channel_renumbering`, `cross_issues`, `with_columns`. They
  forward the grid operations, and `grid()` hands the grid to code that
  wants the cells.
- **Whether the first record is a header is the one interpretation a grid
  makes**, and it makes it the simple way: `Grid::parse` takes the first
  record as the header. Deciding it from the column vocabulary — what
  `docs/spec/format.md` §2 specifies, and what lets a headerless file be
  read positionally with `header_assumed` — stays the typed tables'.

The header and the data rows stay in separate fields, so no index
arithmetic appears anywhere: a data-row index means the same thing in
`Grid`, in `ChTable`, and in an `Issue`.

## Alternatives rejected

- **`Grid` holding every record with no header notion**, the typed tables
  offsetting into it by one: the honest factoring of "a grid is a grid",
  and it puts a `+ 1` on every row index in the crate. The off-by-one
  class of defect is the one this crate has spent the most time on.
- **A trait both tables implement**: gives generic code a way in without
  giving the consumer a type to name, and leaves the duplication where it
  was.
- **A heuristic for the header** (a first record that parses as data is
  data): guessing, where the typed tables already know.
- **Keeping the macro**: it generated a public API into two types from a
  place neither documented; the type it was hiding is the one consumers
  asked for.

## Consequences

- The `grid_api!` macro is gone. A small `grid_delegates!` remains, so
  each table's forwarding methods carry their own documentation rather
  than sending a reader one indirection further.
- The Table JSON of `docs/spec/interchange.md` §2 is the grid's; the
  tables forward it.
- Extracting this surfaced a defect the earlier tests could not see:
  `parse_bytes` stripped the byte-order mark before the text was parsed,
  so a file fed as bytes was written back without the mark it had. The
  mark is valid UTF-8 and the reader already ignores it, so `decode_utf8`
  no longer strips it and the text path records it as it does for any
  other file.
