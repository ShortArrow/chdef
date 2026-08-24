# Diagnostics

🌐 **English** | [日本語](./diagnostics.jp.md)

Implemented (0.0.3): the `Issue` type with the fields of §2,
`Parsed { value, issues }` as the return shape of every loader
(`build_layout` included), and every code below. The cross-file codes come
from `build_layout`, and `layout_exceeds_capacity` from
`ChannelLayout::check_capacity`.

## 1. Principles

- A per-row problem never stops loading. Every readable row is read and the
  problem is returned as an **Issue**.
- Loading stops only when the file cannot be opened (I/O) or the CSV is
  structurally broken (an unterminated quote). Those are errors.
- An Issue carries a row and a column. The row is the **0-based data row**
  (header excluded; it maps directly onto a grid row), the column is the
  0-based column position. An Issue not tied to a row has no row.
- An Issue that fires on every row is not deduplicated (the consumer
  aggregates).

## 2. Issue

```
Issue {
    code,
    row: Option<usize>, col: Option<usize>,     // where in the file
    channel: Option<u32>, bit: Option<u8>,      // what it is about
    found: Option<String>, used: Option<String>,// the values
    message: String,                            // the same, in English
}
```

Everything a consumer needs to write its own sentence is a field:

- `code` is a stable ASCII identifier; consumers key localisation and
  filtering on it.
- `found` is the value chdef could not use, spelled as the file spells it
  — a raw value keeps its cell's notation, so `0x1FF` comes back as
  `0x1FF` and `511` as `511`. It is the one fact parsing throws away.
- `used` is the value chdef put in its place, where it substituted one.
- `channel` and `bit` say which channel, or which bit of which channel,
  the finding is about — the only way to name a finding that carries no
  row.
- A field the finding has nothing to put in is absent, never invented.

`message` is an English rendering of the same facts, for a log and for a
reader who wants one. **Its wording is not part of the contract and may
change in any release**; a consumer that builds its own sentence reads
the fields.

| code | row | Meaning / behaviour |
|---|---|---|
| `header_assumed` | — | No header, or no `number` column; the canonical order was assumed for the columns present (the first 9 of a CH CSV, the 5 of a BF CSV) |
| `channel_number_invalid` | yes | `number` is not an integer / ≤ 0. Row skipped |
| `channel_duplicate` | yes | The same `number` already exists. Layout uses the first row only |
| `bytes_assumed` | yes | `bytes` empty / non-integer. Took the type width, or 2 |
| `bytes_out_of_range` | yes | `bytes` outside 1–8. Clamped to 1–8 |
| `type_assumed` | yes | `type` empty / unknown. Took `UI` |
| `type_width_mismatch` | yes | The width suffix of `type` disagrees with `bytes`. Used `bytes` |
| `lsb_invalid` | yes | `lsb` is NaN / infinite. Took 1 |
| `offset_invalid` | yes | `offset` is not a number. Took 0 |
| `default_invalid` | yes | `default` is neither an integer nor `0x`. Treated as unspecified |
| `raw_display_with_lsb` | yes | The channel shows its raw value (`format` is `HEX`) while `lsb` is not 1, so the number shown is not the physical quantity |
| `raw_out_of_range` | yes² | A raw value — a `default`, a `min` / `max`, or one handed to `encode` — exceeds the channel's width. Used the low bits only |
| `min_invalid` | yes | `min` is neither a number nor `0x`. Treated as unspecified |
| `max_invalid` | yes | `max` is neither a number nor `0x`. Treated as unspecified |
| `min_max_swapped` | yes | The resolved `min` exceeds the resolved `max`. Both kept; the range matches nothing |
| `bf_parent_invalid` | yes | BF `number` is not an integer. Row skipped |
| `bf_bit_invalid` | yes | `bit` is not an integer. Row skipped |
| `bf_bit_out_of_range` | yes / —¹ | `bit` ≥ 64, the widest a channel can be, from the reader with its row; or `bit` ≥ the parent's own width, from the layout without one |
| `bf_parent_not_bitfield` | —¹ | Parent channel missing, or its `type` is not `BF`. Row skipped by the layout |
| `bf_default_invalid` | yes | BF `default` is not `0` / `1`. Treated as unspecified |
| `bf_duplicate` | yes | The same `(number, bit)` already exists. First row only |
| `layout_exceeds_capacity` | — | `total_bytes` exceeds `capacity` |
| `encode_unknown_channel` | — | An encode value names a channel the layout does not have. Ignored |
| `encode_value_invalid` | — | An encode value is NaN / infinite. The channel default was used |

¹ Rowless from `build_layout` (typed rows carry no file coordinates); the
`channel` and `bit` fields name what it is about, and
`BfTable::cross_issues` reports the same finding with the grid row and
column for editors.

² Rowless when it comes from `encode`, which is handed values, not rows.

## 3. Errors

```
ChdefError::Io { path, source }         // the file could not be read
ChdefError::CsvParse { line, message }  // structural (an unterminated quote)
ChdefError::Encoding { valid_up_to }    // bytes that are not UTF-8
```

`line` is the **1-based line of the file**, not the 0-based data row an
Issue carries: a structurally broken file has no rows to number yet.
`Encoding` comes from the `parse_*_csv_bytes` entry points; a file read
through `load_*_csv` that is not UTF-8 fails as `Io`, since the read
itself is what refuses it.

Unlike per-row Issues, an error returns no result. This makes "swap in the
new definition only if it loaded completely" (otherwise keep the previous
one) easy for consumers.
