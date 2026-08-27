# ABI

🌐 **English** | [日本語](./abi.jp.md)

Implemented (0.0.15): everything below — the layout, the conversions, the
named bits, the grid, the value notation and the diagnostics, through the
C ABI of `chdef-capi` and the .NET binding built on it. TypeScript does
not reach these rules through this ABI; the WebAssembly binding over the
crate carries them instead, under the names §3 lists beside the others.

## 1. What the ABI carries

**A consumer reaching chdef through the ABI never has to reimplement a
rule this specification states.** That criterion draws the line in both
directions, and it decides what the ABI grows next.

Carried, because a consumer that writes it holds a second implementation
of a rule that has one home:

| Rule | Document | Carried by |
|---|---|---|
| Position, width, total, capacity | [layout.md](./layout.md) | the layout calls |
| Raw ↔ physical, sign, clamping, byte order | [conversion.md](./conversion.md) §1–§3 | encode / decode |
| BF default merging, named bits of a reading | [conversion.md](./conversion.md) §4, §6 | the bit calls |
| Which of the two a reading shows, and its text | [conversion.md](./conversion.md) §7 | the reading calls |
| `0x` is raw, anything else is physical | [format.md](./format.md) §3 | the value-notation call |
| Which header spelling denotes which column | [format.md](./format.md) §2 | the vocabulary calls |
| The file as cells, and writing it back unchanged | [editing.md](./editing.md) | the grid calls |
| Codes, messages, the row and column they point at | [diagnostics.md](./diagnostics.md) | the diagnostics calls |

Not carried, because a consumer writes it whatever chdef does: editing
UI, undo history, save orchestration, and the presentation choices of the
specification index's out-of-scope list. An ABI that carried these would
be deciding for the application.

The grid is exposed **as cells**, not as a typed editing API: cells are
what the round-trip guarantee of [editing.md §2](./editing.md#2-round-trip) is
about, and a consumer displaying a definition file needs no column
vocabulary to do it.

## 2. Calling conventions

- **Statuses are `int32_t`.** `CHDEF_OK` is `0`, every failure is
  negative, and `CHDEF_PANIC` is returned rather than letting a panic
  cross `extern "C"`. A call that returns a length instead returns `0`
  where it would have failed.
- **No enumeration is exported.** A category — a data type, an issue
  code, a display format — is carried as an ASCII string, so adding one is
  not an ABI break and a caller that does not know it still has something
  to show.
- **Handles are opaque and tagged.** A handle carries a tag its creating
  call sets, so a null pointer, or a handle of one kind passed where
  another was expected, is reported as `CHDEF_ERR_HANDLE` rather than
  dereferenced as the wrong type. A null handle is ignored by its `_free`.
- **Using a handle after freeing it is undefined**, as it is for any C
  pointer: the memory is the allocator's again, and no tag survives in it
  reliably. Freeing clears the tag before releasing the memory, so a stale
  handle is *often* caught — that is a courtesy on the way to a bug, not a
  guarantee, and it is not contracted. The same applies to freeing twice.
- **Text is written into the caller's buffer, never allocated for it.**
  Every text call writes UTF-8, always terminates, and returns the length
  the value needs — so a caller asks with a capacity of `0`, allocates,
  and asks again. The library never hands out a pointer the caller must
  free.
- **Arrays are filled the same way**: the count is written first, and a
  buffer too small is `CHDEF_ERR_BUFFER` with nothing written.
- **Every index is 0-based**, and out of range is `CHDEF_ERR_INDEX` —
  never a panic, never a partial write.

The C header (`crates/chdef-capi/include/chdef.h`) is the authoritative
list of declarations; a test in this repository fails if a symbol exists
without one.

## 3. The surface

Six groups, named after what they carry:

- **Layout** — parse a CH and a BF CSV into a layout, ask its total and
  its channels, set byte order and capacity, check the capacity.
- **Conversion** — encode values into a frame, decode a frame into
  readings, convert one value either way, ask whether a width holds a
  value and what range a channel declares, and read one reading's
  displayed value or rendered text.
- **Bits** — a channel's named bits (number, name, memo, and the
  protocol-spec default it carries or its absence), and the bits of a
  decoded frame with the value each holds.
- **Grid** — parse definition bytes into cells with the vocabulary that
  names its columns, read the header and any cell, set a cell, insert /
  append / remove a row, write the file back, and ask what is wrong with
  the cells as they stand.
- **Value notation** — read the text form of a value into the value it
  denotes.
- **Vocabulary** — the canonical column names, and a vocabulary built from
  them that a parse reads its headers with. A column is identified by its
  canonical **name**, not a number, so adding one to the format is not an
  ABI break.
- **Diagnostics** — the count, the numbers and the text of each finding.

A frame's bits are decoded in one pass, not one call per bit: the count
comes from the layout and one call fills the array, so reading every bit
of a frame costs what reading every channel costs.

A grid is read with the rule the layout uses: the first record is the
header when it names `number` in the vocabulary given, and a file whose
first record does not is read positionally, with no header. Row numbers
in a finding therefore mean the same cells whichever call produced them.

### The same surface, by name

One row per operation, and its name in each binding. A consumer reaching
chdef from any of the three finds every rule above, under a name that
follows the conventions of its language. A test in the repository reads
this table and fails when a binding lacks a name it is given here.

| Operation | C | .NET | JavaScript |
|---|---|---|---|
| Read definitions into a layout | `chdef_layout_parse_with` | `Definitions.Parse` | `Definitions.parse` |
| What was found while reading | `chdef_issue_at` | `Definitions.Issues` | `Definitions.issues` |
| The data length of the frame | `chdef_layout_total_bytes` | `Definitions.TotalBytes` | `Definitions.totalBytes` |
| The channels, in frame order | `chdef_layout_channel_at` | `Definitions.Channels` | `Definitions.channels` |
| Byte order | `chdef_layout_set_endian` | `Definitions.Endian` | `Definitions.endian` |
| Byte capacity | `chdef_layout_set_capacity` | `Definitions.Capacity` | `Definitions.capacity` |
| Channel capacity | `chdef_layout_set_channel_capacity` | `Definitions.ChannelCapacity` | `Definitions.channelCapacity` |
| Whether the frame fits the capacity | `chdef_layout_limits_exceeded` | `Definitions.LimitsExceeded` | `Definitions.limitsExceeded` |
| Values into a frame | `chdef_encode` | `Definitions.Encode` | `Definitions.encode` |
| A frame into readings | `chdef_decode` | `Definitions.Decode` | `Definitions.decode` |
| Fill the derived channels | `chdef_seal` | `Definitions.Seal` | `Definitions.seal` |
| Which derived channels disagree | `chdef_derived_mismatches` | `Definitions.DerivedMismatches` | `Definitions.derivedMismatches` |
| The bytes a derived channel covers | `chdef_covered_bytes` | `Definitions.CoveredBytes` | `Definitions.coveredBytes` |
| The recipes known by name | `chdef_recipe_name` | `Definitions.Recipes` | `recipes` |
| One physical value to its raw pattern | `chdef_layout_channel_to_raw` | `Definitions.ToRaw` | `Definitions.toRaw` |
| One raw pattern to its physical value | `chdef_layout_channel_to_value` | `Definitions.ToValue` | `Definitions.toValue` |
| Whether the width holds a value | `chdef_layout_channel_fits_width` | `Definitions.FitsWidth` | `Definitions.fitsWidth` |
| The declared range, as physical values | `chdef_layout_channel_range` | `Definitions.RangeOf` | `Definitions.rangeOf` |
| Which values fall outside their range | `chdef_values_out_of_range` | `Definitions.ValuesOutOfRange` | `Definitions.valuesOutOfRange` |
| Which readings fall outside their range | `chdef_readings_out_of_range` | `Definitions.ReadingsOutOfRange` | `Definitions.readingsOutOfRange` |
| Which reading the format column shows | `chdef_layout_channel_displayed` | `Definitions.Displayed` | `Definitions.displayed` |
| The default text of a reading | `chdef_layout_channel_render` | `Definitions.Render` | `Definitions.render` |
| The named bits of a channel | `chdef_layout_bit_at` | `Channel.Bits` | `Channel.bits` |
| The bits of a reading | `chdef_decode_bits` | `Reading.Bits` | `Reading.bits` |
| The text form of a value | `chdef_value_parse` | `Value.Parse` | `parseValue` |
| Read a file as cells | `chdef_grid_parse_with` | `Grid.Parse` | `Table.parse` |
| The header cells | `chdef_grid_header_at` | `Grid.Header` | `Table.header` |
| How many data rows | `chdef_grid_row_count` | `Grid.RowCount` | `Table.rowCount` |
| How many cells a row has | `chdef_grid_col_count` | `Grid.ColumnCount` | `Table.columnCount` |
| One cell | `chdef_grid_cell` | `Grid.Cell` | `Table.cell` |
| Overwrite one cell | `chdef_grid_set_cell` | `Grid.SetCell` | `Table.setCell` |
| Insert a row | `chdef_grid_insert_row` | `Grid.InsertRow` | `Table.insertRow` |
| Append a row | `chdef_grid_append_row` | `Grid.AppendRow` | `Table.appendRow` |
| Remove a row | `chdef_grid_remove_row` | `Grid.RemoveRow` | `Table.removeRow` |
| Write the file back | `chdef_grid_to_csv` | `Grid.ToCsv` | `Table.toCsv` |
| What is wrong with the cells | `chdef_grid_issues` | `Grid.Issues` | `Table.issues` |
| Which defaults leave their own range | `chdef_grid_defaults_out_of_range` | `Grid.DefaultsOutOfRange` | `Table.defaultsOutOfRange` |
| The empty vocabulary | `chdef_vocabulary_new` | `ColumnVocabulary.Create` | `new ColumnVocabulary()` |
| The Japanese vocabulary | `chdef_vocabulary_japanese` | `ColumnVocabulary.Japanese` | `ColumnVocabulary.japanese` |
| Teach a CH spelling | `chdef_vocabulary_teach` | `ColumnVocabulary.Ch` | `ColumnVocabulary.ch` |
| Teach a BF spelling | `chdef_vocabulary_teach` | `ColumnVocabulary.Bf` | `ColumnVocabulary.bf` |
| The canonical CH column names | `chdef_column_name` | `ColumnVocabulary.ChColumnNames` | `ColumnVocabulary.chColumns` |
| The canonical BF column names | `chdef_column_name` | `ColumnVocabulary.BfColumnNames` | `ColumnVocabulary.bfColumns` |
| Every Issue code a build can report | `chdef_issue_code_name` | `IssueCode.All` | `issueCodes` |

## 4. Version

`chdef_abi_version()` returns `CHDEF_ABI_VERSION`, which **increments
whenever a symbol is added or changed**. A caller checks that it is
**at least** the value its declarations were written for. Symbols are
added and never withdrawn, so a newer library serves an older caller; the
check catches the reverse, which is a caller asking for symbols that are
not there.

The .NET package carries the native library for every runtime it
supports, so a consumer taking that route cannot pair the two wrongly.
