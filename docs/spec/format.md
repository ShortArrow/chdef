# File format and columns

🌐 **English** | [日本語](./format.jp.md)

Implemented (0.0.15): BOM stripping; column identification by header name
in English or Japanese, with the 9-column positional fallback and any
spellings a reader is taught (`ColumnAliases`); blank rows and `#` rows;
every column interpretation below with its Issues; `parse_*_csv_bytes` for
byte input; `load_*_csv` over any `AsRef<Path>`. Writing is `to_csv`, with
the round trip of [editing.md §2](./editing.md#2-round-trip); a file chdef
creates names the columns and the language the caller asked for
(`with_columns`).

## 1. File

| Item | Read | Write |
|---|---|---|
| Encoding | UTF-8. One or more leading BOMs (`EF BB BF`) are all ignored | As the file was read; UTF-8 with a BOM for a file chdef creates (so spreadsheet software does not guess another encoding) |
| Record separator | Both `\r\n` and `\n` are accepted | As the file was read; `\r\n` for a file chdef creates |
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

Columns are identified **by header name**, in three steps:

1. The header row is split into cells (§1) — no meaning is attached yet.
2. Each cell is mapped to a column through the **vocabulary in use**.
3. The column is then interpreted as this document says.

A column has one **canonical name**: lower-case ASCII, listed in §3 and
§4. That name is the column identity — in this specification, in the JSON
of [interchange.md](./interchange.md), and in every API chdef offers.
Anything else a header may say is vocabulary.

**A vocabulary is data the caller supplies**, not a language chdef knows.
It maps spellings to columns for reading, and names the spelling to write
for a file chdef creates: the first spelling it teaches for a column is
the one written. chdef ships one (`ColumnVocabulary::japanese()`, the
table below) and it has no privilege over one a caller builds; a header
in any language is read by teaching its spellings.

Whatever the vocabulary, chdef also recognises **the canonical name and
the variants listed in §3 and §4** — they are the identity, and a
vocabulary adds to them rather than replacing them. Cells are trimmed and
matched case-insensitively. A column absent from the header is
"unspecified" and is not an error. **Unknown columns are preserved and
written back as they were.**

If there is no header row, or no `number` column can be found, the columns
are **assumed** to be in canonical order — the first 9 of a CH CSV, or the
5 of a BF CSV — and one Issue `header_assumed` is reported.

Header names are written back exactly as they were read. A newly created
file names the columns the caller asked for, spelled by the vocabulary the
caller asked for (`ChTable::with_columns`); the default is every column in
canonical order under the canonical names.

Two rules keep the format a format whatever a caller teaches:

- **A vocabulary only adds.** A cell is matched against the canonical
  names and variants first, so a canonical name always means what this
  document says. Teaching `number` to mean something else does nothing.
- **No vocabulary appears in the golden vectors**
  ([interchange.md §3](./interchange.md)). Conformance is defined on the
  canonical names alone, so an implementation in another language is
  obliged to offer no vocabulary at all.

### The Japanese vocabulary

The spellings of the definition files this format was extracted from,
shipped as `ColumnVocabulary::japanese()`. The first spelling of each row
is the one written.

| Column | Spellings |
|---|---|
| `number` | `番号` |
| `bytes` | `バイト数` |
| `bits` | `ビット数` |
| `section` | `セクション名` |
| `name` | `メッセージ名称`, `信号名称` |
| `type` | `型`, `データ型` |
| `lsb` | `LSB`, `スケール` |
| `offset` | `オフセット`, `基準値` |
| `unit` | `単位` |
| `min` | `値(最小)`, `最小値` |
| `max` | `値(最大)`, `最大値` |
| `default` | `値(デフォルト)`, `デフォルト値` |
| `memo` | `備考` |
| `var` | `変数名` |
| `format` | `表示形式` |
| `kind` | `種別` |
| `derived` | `算出` |
| `favorite` | `お気に入り` |

For a BF CSV:

| Column | Spellings |
|---|---|
| `number` | `番号` |
| `bit` | `BIT番号` |
| `name` | `メッセージ名称`, `信号名称` |
| `default` | `値(デフォルト)`, `デフォルト値` |
| `memo` | `備考` |

## 3. CH CSV

Canonical header (17-column form):

```
number,bytes,bits,section,name,type,lsb,offset,unit,min,max,default,memo,var,format,favorite,kind
```

The same columns spelled by the Japanese vocabulary of §2:

```
番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位,値(最小),値(最大),値(デフォルト),備考,変数名,表示形式,お気に入り,種別
```

The 9–10-column form (first 9 columns plus `default`) is read by the same
rules.

| Column | Variants | Required | Interpretation |
|---|---|---|---|
| `number` | `no`, `CH`, `ChNumber` | **yes** | Integer ≥ 1. Empty / non-integer / ≤ 0 → the **whole row is skipped** (Issue `channel_number_invalid`). All-empty rows and `#` rows are skipped without an Issue. No upper bound (u32) |
| `bytes` | — | no | Integer 1–8. Empty / non-integer → the width of `type` (below), else 2 (Issue `bytes_assumed`). Out of range → clamped to 1–8 (Issue `bytes_out_of_range`) |
| `bits` | — | no | Not read. Preserved and written back |
| `section` | — | no | String. Empty means no section |
| `name` | `SignalName` | no | String |
| `type` | `DataType` | no | Two-letter prefix (case-insensitive) `UI` / `SI` / `BF` plus an optional width suffix (`UI8` `SI16` `UI32` `SI64` …). Empty / unknown → `UI` (Issue `type_assumed`). The width always comes from `bytes`; if the suffix disagrees, `bytes` wins (Issue `type_width_mismatch`) |
| `lsb` | `Scale` | no | Real number. Empty / `0` → `1`. Any other finite value is used as is (negative allowed). NaN / infinite → `1` (Issue `lsb_invalid`) |
| `offset` | — | no | Real number. Empty → `0`. Not a number → `0` (Issue `offset_invalid`) |
| `unit` | — | no | String |
| `min`, `max` | — | no | Empty → unspecified. A number → **physical** bound (finite f64). `0x` / `0X` → **raw** bound, width-checked like `default` (Issue `raw_out_of_range`, low bits kept). Anything else → unspecified (Issue `min_invalid` / `max_invalid`). A resolved `min` above `max` → both kept (Issue `min_max_swapped`). Never applied by a conversion — `range_contains` / `clamp_to_range` are the explicit queries |
| `default` | `DefaultValue` | no | Empty → unspecified. `0x` / `0X` prefix → hexadecimal raw value. Anything else → decimal **raw value** (integer). Either way it is as wide as the channel; past that width → the low bits are kept (Issue `raw_out_of_range`). Neither notation → unspecified (Issue `default_invalid`). For `BF` channels, the BF CSV overrides it bit by bit ([conversion.md](./conversion.md)) |
| `memo` | `Description` | no | String |
| `var` | `variable` | no | String. Preserved only |
| `format` | `DisplayFormat` | no | `DEC` / `HEX` (case-insensitive). Empty / unknown → `DEC`. `HEX` with `lsb` ≠ 1 → Issue `raw_display_with_lsb`. What the column selects is **which reading is shown** — the physical value or the raw one — not the base it is printed in ([conversion.md §7](./conversion.md)). It never affects a conversion |
| `kind` | — | no | `plain` / `const` / `counter` / `derived`, case-insensitive and trimmed. Empty → `plain`. Anything else → `plain` (Issue `kind_assumed`). It records **who decides this channel's value** and nothing else: `encode` behaves identically whatever it says, and chdef never fills a channel because of it (§5) |
| `derived` | — | no | How a `derived` channel is computed (§6). Read only for `kind` = `derived`, and ignored otherwise. Unreadable → the channel keeps its `default` and nothing is computed (Issue `derived_invalid`) |
| `favorite` | `IsFavorite` | no | `1` or `true` (case-insensitive) → true, anything else → false. Written as `1` / `0` |

Type prefix and width: the width is `bytes × 8` bits. `UI` is an unsigned
integer, `SI` a two's-complement signed integer, `BF` a bag of bits
(unsigned as a value).

## 4. BF CSV

```
number,bit,name,default,memo
```

| Column | Variants | Required | Interpretation |
|---|---|---|---|
| `number` | `no`, `CH` | **yes** | `number` of the parent channel. Empty / non-integer → the whole row is skipped (Issue `bf_parent_invalid`) |
| `bit` | `BitNumber` | **yes** | Integer ≥ 0 and below the parent width `bytes × 8`. Non-integer → row skipped (Issue `bf_bit_invalid`). ≥ width → row skipped (Issue `bf_bit_out_of_range`). Parent undefined, or parent `type` is not `BF` → row skipped (Issue `bf_parent_not_bitfield`) |
| `name` | `SignalName` | no | String |
| `default` | — | no | Only `0` / `1` are valid. Empty → unspecified (the parent's default bit is kept). Anything else → unspecified (Issue `bf_default_invalid`) |
| `memo` | — | no | String |

A BF CSV only needs rows for channels whose `type` is `BF`; it does not have to
list every channel. One row is one bit. Bit ranges (several bits such as
`3:1`) are not expressible in 0.0.x.

## 5. Who fills a channel

`kind` records where a channel's value comes from, so that the fact travels
with the row. Inserting a channel renumbers every channel after it; a
constant held in code, or a table in another file, goes silently wrong at
that moment, and a cell cannot.

| `kind` | Meaning |
|---|---|
| `plain` | The caller supplies the value, or the channel takes its `default`. |
| `const` | The value is the `default` and does not change from frame to frame — a sync word, a protocol version. |
| `counter` | The caller supplies a number that advances every frame. |
| `derived` | chdef computes it from the rest of the frame, by the recipe in the `derived` column (§6). |

**chdef reads the column, carries it, writes it back, and reports what it
could not read**, and `encode` produces the same bytes whatever `kind`
says. `derived` is the one kind chdef can compute, and it still does not
compute it in `encode`: sealing a frame is a call of its own (§6). Overriding a `const` channel
is not an Issue: what a caller may send is the caller's to decide.

A `counter` is not advanced by chdef, because a counter belongs to the
line that sends the frames and one definition may be shared by several
lines, each with its own running number. **The caller also wraps it**: a
raw value wider than the channel keeps its low bits (§3, Issue
`raw_out_of_range`), while a physical value wider than it saturates
([conversion.md §2](./conversion.md)), so a counter is passed as a raw
value already reduced to the channel's width.

Values beyond these three may appear; a reader that does not know one
treats it as `plain` and says so, so a file written for a later chdef
still loads.

## 6. Derived channels

A channel whose `kind` is `derived` is computed from the rest of the
frame. The `derived` column says how. Today one recipe is defined:

```
crc16/x25 1..7
^^^^^^^^^ ^^^^
recipe    the channels it covers, both ends included
```

- **The range names channels, not bytes**, by their `number`. Inserting a
  channel renumbers the ones after it and the range follows, which a byte
  range would not. The channels are covered in layout order; a `number`
  the layout does not hold is an Issue and the recipe computes nothing.
- **The range is required, and there is no default.** A frame laid out as
  sync, length, data, CRC may cover the data alone, the length and the
  data, or everything before the CRC — which of those a device means is a
  property of its protocol, not of CRC. A default would be right often and
  silently wrong otherwise, and a CRC that is silently wrong shows up as
  hardware discarding frames with no reason given. A recipe without a
  range is `derived_invalid`.
- **Spans may be listed** when the covered channels are not one run:
  `crc16/x25 2..3,5..7`. Each span is `low..high` with both ends included,
  and they are covered left to right as written.
- **A recipe is six numbers**, the model every CRC is described by:
  width, polynomial, initial value, whether the input is reflected,
  whether the output is reflected, and the value XORed at the end. Written
  out, the line above is

  ```
  crc16 poly=0x1021 init=0xFFFF refin=1 refout=1 xorout=0xFFFF 1..7
  ```

  chdef ships the catalogued variants as names — `crc16/x25` is exactly
  the six numbers above — and a name has no standing the numbers lack. A
  file for a device whose CRC is in no catalogue writes the numbers.
- Anything else in the cell is an Issue `derived_invalid`; the channel
  keeps its `default` and nothing is computed.

The names this chdef ships, each with the numbers it stands for and the
check value that identifies it — the CRC of the ASCII bytes `123456789`,
the self-test every CRC catalogue prints:

| name | width | poly | init | refin | refout | xorout | check |
|---|---|---|---|---|---|---|---|
| `crc16/x25` | 16 | `0x1021` | `0xFFFF` | yes | yes | `0xFFFF` | `0x906E` |
| `crc16/ibm-3740` | 16 | `0x1021` | `0xFFFF` | no | no | `0x0000` | `0x29B1` |
| `crc16/kermit` | 16 | `0x1021` | `0x0000` | yes | yes | `0x0000` | `0x2189` |
| `crc16/xmodem` | 16 | `0x1021` | `0x0000` | no | no | `0x0000` | `0x31C3` |
| `crc8/smbus` | 8 | `0x07` | `0x00` | no | no | `0x00` | `0xF4` |
| `crc32/iso-hdlc` | 32 | `0x04C11DB7` | `0xFFFFFFFF` | yes | yes | `0xFFFFFFFF` | `0xCBF43926` |

A device whose CRC is in no catalogue writes the numbers instead, and one
using something that is not a CRC at all is not blocked either: the
coverage is still read, and `covered_bytes` hands over exactly the bytes
it names.

### Sealing and checking

`encode` never fills a derived channel. **Sealing is a call of its own**,
`ChannelLayout::seal`, which fills every derived channel of a frame in
layout order. A frame is sealed once, after every other value is in place,
because a recipe reads the bytes as they will be sent.

Going the other way, `derived_mismatches` reports Issue `derived_mismatch`
for every derived channel whose stored value disagrees with the recipe —
the check a receiver makes, and the one place chdef says a frame is wrong
rather than merely unusual.

Neither is applied by `encode` or `decode`, and neither is remembered.
