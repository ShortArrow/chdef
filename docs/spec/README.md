# chdef specification

🌐 **English** | [日本語](./README.jp.md)

The contract for the definition files chdef reads, the layout it computes,
the conversions it performs, and the diagnostics it returns. Each document
starts with the scope that the current release actually implements.

| Document | Content |
|---|---|
| [format.md](./format.md) | File format (encoding, line endings, quoting), the columns of a CH CSV / BF CSV and how each cell is interpreted |
| [layout.md](./layout.md) | Rows → channels → layout (position, width, total), duplicates, gaps, capacity |
| [conversion.md](./conversion.md) | Raw ↔ physical conversion, sign extension, rounding, BF default merging, encode / decode |
| [diagnostics.md](./diagnostics.md) | Diagnostics (Issue): granularity, codes, messages, and the line between an Issue and a fatal error |
| [editing.md](./editing.md) | The Table stage as the editing substrate: cell / row edits, typed insertion, renumbering, the round-trip guarantee |
| [interchange.md](./interchange.md) | JSON output shape and the golden-vector format |

## Terms

- **CH (channel)**: one contiguous run of bytes inside a frame. Identified by
  `number`; its width is `bytes`.
- **BF (bit field)**: one bit inside a channel whose `type` is `BF`.
  Identified by `(parent number, bit)`.
- **Raw value**: the bit pattern on the wire read as an integer of the
  channel's width.
- **Physical value**: `raw × lsb + offset`.
- **Layout**: the position of every channel when the definitions are packed
  from the start of the frame in definition order, plus the total byte count.
- **Issue**: a diagnostic that points at a row / column. It never stops
  loading.

Columns are referred to by their English spelling throughout; every column
also has a Japanese canonical spelling, and the reader accepts both (see
[format.md §2](./format.md#2-identifying-columns)).

## Out of scope (what chdef does not own)

- Presentation (DEC / HEX rendering, digit counts, colours), editing UI,
  save orchestration.
- Transport (UDP / shared memory / serial), packet headers, packet-number
  rewriting, merging several CSVs into one frame.
- Where definition files live, path settings, when to reload.
- Real-device definition data. Every sample and golden vector in this
  repository is synthetic.
