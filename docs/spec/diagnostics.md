# Diagnostics

🌐 **English** | [日本語](./diagnostics.jp.md)

Implemented (0.0.2): the `Issue` type, `Parsed { value, issues }` as the
return shape of every loader (`build_layout` included), and every code
below. The cross-file codes come from `build_layout`, and
`layout_exceeds_capacity` from `ChannelLayout::check_capacity`.

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
Issue { code, row: Option<usize>, col: Option<usize>, message: String }
```

`code` is a stable ASCII identifier; consumers key localisation and
filtering on it. `message` is one English sentence that says what was
found and what chdef did about it.

| code | row | Meaning / behaviour |
|---|---|---|
| `header_assumed` | — | No header, or no `number` column; the first 9 columns were taken in canonical order |
| `channel_number_invalid` | yes | `number` is not an integer / ≤ 0. Row skipped |
| `channel_duplicate` | yes | The same `number` already exists. Layout uses the first row only |
| `bytes_assumed` | yes | `bytes` empty / non-integer. Took the type width, or 2 |
| `bytes_out_of_range` | yes | `bytes` outside 1–8. Clamped to 1–8 |
| `type_assumed` | yes | `type` empty / unknown. Took `UI` |
| `type_width_mismatch` | yes | The width suffix of `type` disagrees with `bytes`. Used `bytes` |
| `lsb_invalid` | yes | `lsb` is NaN / infinite. Took 1 |
| `offset_invalid` | yes | `offset` is not a number. Took 0 |
| `default_invalid` | yes | `default` is neither an integer nor `0x`. Treated as unspecified |
| `hex_with_lsb` | yes | `format` is `HEX` but `lsb` is not 1 |
| `raw_out_of_range` | yes | A `0x` raw value exceeds the width. Used the low bits only |
| `min_invalid` | yes | `min` is neither a number nor `0x`. Treated as unspecified |
| `max_invalid` | yes | `max` is neither a number nor `0x`. Treated as unspecified |
| `min_max_swapped` | yes | The resolved `min` exceeds the resolved `max`. Both kept; the range matches nothing |
| `bf_parent_invalid` | yes | BF `number` is not an integer. Row skipped |
| `bf_bit_invalid` | yes | `bit` is not an integer. Row skipped |
| `bf_bit_out_of_range` | —¹ | `bit` ≥ parent width. Row skipped by the layout; the message names `(number, bit)` |
| `bf_parent_not_bitfield` | —¹ | Parent channel missing, or its `type` is not `BF`. Row skipped by the layout; the message names `(number, bit)` |
| `bf_default_invalid` | yes | BF `default` is not `0` / `1`. Treated as unspecified |
| `bf_duplicate` | yes | The same `(number, bit)` already exists. First row only |
| `layout_exceeds_capacity` | — | `total_bytes` exceeds `capacity` |
| `encode_unknown_channel` | — | An encode value names a channel the layout does not have. Ignored |
| `encode_value_invalid` | — | An encode value is NaN / infinite. The channel default was used |

¹ Rowless from `build_layout` (typed rows carry no file coordinates);
`BfTable::cross_issues` reports the same finding with the grid row and
column for editors.

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
