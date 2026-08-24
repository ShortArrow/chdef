# File format and columns

🌐 **English** | [日本語](./format.jp.md)

Implemented (0.0.2): BOM stripping; column identification by header name
in English or Japanese with the 9-column positional fallback; blank rows
and `#` rows; every column interpretation below with its Issues;
`parse_*_csv_bytes` for byte input; `load_*_csv` over any `AsRef<Path>`.
Writing is `ChTable::to_csv` /
`BfTable::to_csv` with the cell-level round trip of
[editing.md §2](./editing.md#2-round-trip).

## 1. File

| Item | Read | Write |
|---|---|---|
| Encoding | UTF-8. One or more leading BOMs (`EF BB BF`) are all ignored | UTF-8 with BOM (so spreadsheet software does not guess another encoding) |
| Record separator | Both `\r\n` and `\n` are accepted | `\r\n` |
| Newlines inside a cell | `\n` / `\r\n` inside a quoted cell is part of the cell (RFC 4180) | Written verbatim (`\n` is not normalised to `\r\n`) |
| Quoting | `"..."`; an inner `"` is `""`. A `"` outside quotes is a literal character | A cell containing any of `,` `"` `\r` `\n` is quoted |
| Whitespace | Leading / trailing spaces and tabs of a cell are trimmed before interpretation | Not trimmed (the original text is kept) |
| Blank row | A row whose cells are all empty is skipped (no Issue) | — |
| Comment row | A row whose first cell starts with `#` is skipped (no Issue) | — |
| Empty file / header only | Zero channels, no Issue | — |

**Who decodes**: chdef reads UTF-8 and does not guess. `parse_ch_csv` takes
text the caller has already decoded; `parse_ch_csv_bytes` takes bytes, drops
the BOM and decodes as UTF-8, and reports the offset at which decoding
stopped. A file a spreadsheet wrote in CP932 is the consumer's to decode
before either call.

**Why quote**: if a cell containing a newline is written unquoted, the next
read splits one row into several records, and the trailing fragments become
ghost rows whose `number` cell holds something that is not a number. The
reader drops ghost rows through the `number` check, but the lost `memo` never
comes back.

## 2. Identifying columns

Columns are identified **by header name**. Every column has two canonical
spellings — **English** (lower-case ASCII) and **Japanese** (the original
form) — plus aliases; all of them denote the same column. Header cells are
trimmed and matched case-insensitively, and the two languages may be mixed
in one header. A column absent from the header is "unspecified" and is not
an error. **Unknown columns are preserved and written back as they were.**

If there is no header row, or no `number` column can be found, the first
9 columns are **assumed** to be the canonical CH CSV order (the table below)
for compatibility with the 9-column form, and one Issue `header_assumed`
is reported.

Header names are written back exactly as they were read. A newly created
file uses the English spellings unless the writer is told otherwise.

## 3. CH CSV

Canonical header (16-column form), English and Japanese spellings:

```
number,bytes,bits,section,name,type,lsb,offset,unit,min,max,default,memo,var,format,favorite
番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位,値(最小),値(最大),値(デフォルト),備考,変数名,表示形式,お気に入り
```

The 9–10-column form (first 9 columns plus `default`) is read by the same
rules.

| Column (en / ja) | Aliases | Required | Interpretation |
|---|---|---|---|
| `number` / `番号` | `no`, `CH`, `ChNumber` | **yes** | Integer ≥ 1. Empty / non-integer / ≤ 0 → the **whole row is skipped** (Issue `channel_number_invalid`). All-empty rows and `#` rows are skipped without an Issue. No upper bound (u32) |
| `bytes` / `バイト数` | — | no | Integer 1–8. Empty / non-integer → the width of `type` (below), else 2 (Issue `bytes_assumed`). Out of range → clamped to 1–8 (Issue `bytes_out_of_range`) |
| `bits` / `ビット数` | — | no | Not read. Preserved and written back |
| `section` / `セクション名` | — | no | String. Empty means no section |
| `name` / `メッセージ名称` | `SignalName`, `信号名称` | no | String |
| `type` / `型` | `DataType`, `データ型` | no | Two-letter prefix (case-insensitive) `UI` / `SI` / `BF` plus an optional width suffix (`UI8` `SI16` `UI32` `SI64` …). Empty / unknown → `UI` (Issue `type_assumed`). The width always comes from `bytes`; if the suffix disagrees, `bytes` wins (Issue `type_width_mismatch`) |
| `lsb` / `LSB` | `Scale`, `スケール` | no | Real number. Empty / `0` → `1`. Any other finite value is used as is (negative allowed). NaN / infinite → `1` (Issue `lsb_invalid`) |
| `offset` / `オフセット` | `基準値` | no | Real number. Empty → `0`. Not a number → `0` (Issue `offset_invalid`) |
| `unit` / `単位` | — | no | String |
| `min` / `値(最小)`, `max` / `値(最大)` | `最小値` / `最大値` | no | Empty → unspecified. A number → **physical** bound (finite f64). `0x` / `0X` → **raw** bound, width-checked like `default` (Issue `raw_out_of_range`, low bits kept). Anything else → unspecified (Issue `min_invalid` / `max_invalid`). A resolved `min` above `max` → both kept (Issue `min_max_swapped`). Never applied by a conversion — `range_contains` / `clamp_to_range` are the explicit queries |
| `default` / `値(デフォルト)` | `DefaultValue`, `デフォルト値` | no | Empty → unspecified. `0x` / `0X` prefix → hexadecimal raw value. Anything else → decimal **raw value** (integer). Either way it is as wide as the channel; past that width → the low bits are kept (Issue `raw_out_of_range`). Neither notation → unspecified (Issue `default_invalid`). For `BF` channels, the BF CSV overrides it bit by bit ([conversion.md](./conversion.md)) |
| `memo` / `備考` | `Description` | no | String |
| `var` / `変数名` | `variable` | no | String. Preserved only |
| `format` / `表示形式` | `DisplayFormat` | no | `DEC` / `HEX` (case-insensitive). Empty / unknown → `DEC`. `HEX` with `lsb` ≠ 1 → Issue `hex_with_lsb`. Carried for the consumer to render with; it never affects a conversion |
| `favorite` / `お気に入り` | `IsFavorite` | no | `1` or `true` (case-insensitive) → true, anything else → false. Written as `1` / `0` |

Type prefix and width: the width is `bytes × 8` bits. `UI` is an unsigned
integer, `SI` a two's-complement signed integer, `BF` a bag of bits
(unsigned as a value).

## 4. BF CSV

```
number,bit,name,default,memo
番号,BIT番号,メッセージ名称,値(デフォルト),備考
```

| Column (en / ja) | Aliases | Required | Interpretation |
|---|---|---|---|
| `number` / `番号` | `no`, `CH` | **yes** | `number` of the parent channel. Empty / non-integer → the whole row is skipped (Issue `bf_parent_invalid`) |
| `bit` / `BIT番号` | `BitNumber` | **yes** | Integer ≥ 0 and below the parent width `bytes × 8`. Non-integer → row skipped (Issue `bf_bit_invalid`). ≥ width → row skipped (Issue `bf_bit_out_of_range`). Parent undefined, or parent `type` is not `BF` → row skipped (Issue `bf_parent_not_bitfield`) |
| `name` / `メッセージ名称` | `SignalName`, `信号名称` | no | String |
| `default` / `値(デフォルト)` | `デフォルト値` | no | Only `0` / `1` are valid. Empty → unspecified (the parent's default bit is kept). Anything else → unspecified (Issue `bf_default_invalid`) |
| `memo` / `備考` | — | no | String |

A BF CSV only needs rows for channels whose `type` is `BF`; it does not have to
list every channel. One row is one bit. Bit ranges (several bits such as
`3:1`) are not expressible in 0.0.x.
