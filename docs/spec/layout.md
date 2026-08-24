# Layout

🌐 **English** | [日本語](./layout.jp.md)

Implemented (0.0.6): cumulative positions and total byte count
(`ChannelLayout::channel_offset` / `total_bytes()` / `positions()`, all
computed on demand so an edited width is never stale); the Rows / Layout
split (`parse_*` keeps duplicates, `build_layout` drops them first-wins);
the whole-layout `endian` of §2; the BF cross-checks in `build_layout`,
which returns the layout with those Issues; the `capacity` of §5, carried
on the layout and reported by `check_capacity()`. The Table stage is
`Grid`, `ChTable` and `BfTable` ([editing.md](./editing.md)); whole-frame
encode and decode are [conversion.md](./conversion.md) §5 / §6.

## 1. Three stages

chdef handles a definition in three stages, and every stage is retrievable.

| Stage | Content | Used for |
|---|---|---|
| **Table** | Header and cells as a two-dimensional array, verbatim | Editing UI, preserving unknown columns, writing back |
| **Rows** | Each row interpreted. Every row that was not skipped (duplicates included) plus the Issues | Consumers that want the set of rows itself, e.g. a sequence check |
| **Layout** | Channels with duplicates removed and their positions, BFs, total byte count | encode / decode, display |

## 2. Position

- Positions are not written in the CSV. The position `at` of a channel is
  the cumulative `bytes` from the start in **Rows order** (not ascending
  `number`).
- The total byte count `total_bytes()` is the sum of the widths. It is the
  data length of the frame (the consumer adds any header separately).
- Multi-byte raw values are little-endian by default; a layout can be
  switched to big-endian as a whole via `endian`. This is not written in the
  CSV.

## 3. Duplicates

- When the same `number` appears in several rows, only the **first row** goes
  into the Layout; the rest are reported as Issue `channel_duplicate`. Rows
  keeps all of them.
- The same holds for BF rows with the same `(parent number, bit)` (first
  wins, Issue `bf_duplicate`).

## 4. Gaps

- Gaps in `number` leave the Layout unchanged (channels are packed). Filling
  gaps or enforcing consecutive numbers is the consumer's decision; chdef
  only hands over Rows.

## 5. Capacity

- A layout may carry a `capacity` (the maximum byte count of the data
  part), set with `with_capacity`. `check_capacity()` then reports Issue
  `layout_exceeds_capacity` when the frame does not fit it, and nothing
  when it fits or when the layout carries none. Nothing checks it unless
  asked.

## 6. Type and width

- The width is `bytes` (1–8). `type` carries no width, only the
  interpretation (`UI` / `SI` / `BF`). A definition built in code whose
  `bytes` falls outside 1–8 is measured at the nearest legal width
  (`ChannelDef::width`), the same range the column is clamped to.
- `SI` is sign-extended at its width. `UI` / `BF` are unsigned.
