# Editing

🌐 **English** | [日本語](./editing.jp.md)

Implemented (0.0.2): everything below (`ChTable` / `BfTable`: parse /
`to_csv` / cell and row edits / `insert_channel` /
`insert_channel_renumbering`). Editing **UI**, undo history and save
orchestration stay out of scope — the Table is a value, so a consumer
snapshots it by cloning.

## 1. The Table is what is edited

The Table stage ([layout.md §1](./layout.md#1-three-stages)) holds the
header row and every data row as verbatim cell strings — comment rows,
blank rows and unknown columns included. It is the source of truth during
an editing session. Rows and Layout are derived views: interpret again
after an edit (`channels()` / `bitfields()`, then `build_layout`), the
same way positions and `total_bytes` are always recomputed. Nothing is
cached, so nothing goes stale.

## 2. Round trip

Read → Table → write preserves **rows and cell contents**, not bytes:

- Header spelling, unknown columns, comment rows and rows whose cells are
  all empty survive as they were.
- Quoting is normalised to the write rules ([format.md §1](./format.md#1-file)):
  a cell is quoted only when it holds `,` `"` or a newline; unnecessary
  quotes in the source are dropped.
- The record separator is normalised to `\r\n` and one BOM is written.
- A line with no cells at all (fully empty, no commas) yields no record
  and is not preserved. A skippable-but-present row is spelled `,,,`.

## 3. Edit operations

- `cell` / `set_cell(row, col, value)`: the grid, 0-based, header
  excluded — the same row numbering Issues use. Setting past the end of a
  short row pads it with empty cells.
- `insert_row` / `append_row` / `remove_row`: raw rows, for grid editors.
  All three are total: an index past the end clamps, is ignored, or comes
  back as `None`, never a panic.
- `insert_channel(row_index, &ChannelDef)`: renders a typed definition
  into the columns **this file has**; a field without a column is dropped.
  Rendering: `number` / `bytes` / `default` in decimal, `type` as its
  category only (`UI` / `SI` / `BF` — the width lives in `bytes`),
  `lsb` / `offset` in shortest decimal form, `min` / `max` in their own
  notation (physical as a number, raw as `0x`).

## 4. Cross-file checks on the grid

`BfTable::cross_issues(&channels)` runs the layout's cross-file checks
(`bf_parent_not_bitfield` / `bf_bit_out_of_range`) where the rows still
exist, so each finding carries the grid row and the `number` / `bit`
column — an editor paints the cell instead of a rowless message. Rows
whose `number` / `bit` do not parse are already reported, with rows, by
`bitfields()`.

## 5. Renumbering

`insert_channel_renumbering(row_index, &def, Some(&mut bf_table))` is the
consecutive-numbering insertion: every channel whose `number` ≥
`def.number` moves up by one (its `number` cell is rewritten), BF rows
follow their parents, then the new row is inserted. The returned
`Renumbered { moved }` lists each `(old, new)` pair once, in ascending
order, however many rows carry that number. A channel numbered `u32::MAX`
has nowhere to move to and stays where it is.

References outside the two files — TOML configs, notes, code — are not
chdef's to repair: `moved` is exactly the information a consumer needs to
repair or announce them. Inserting under a free number instead moves
nothing and returns an empty `moved`.
