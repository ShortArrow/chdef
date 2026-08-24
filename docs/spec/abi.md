# ABI

🌐 **English** | [日本語](./abi.jp.md)

Implemented (0.0.6): everything below — the layout, the conversions, the
named bits, the grid, the value notation and the diagnostics, through the
C ABI of `chdef-capi` and the .NET binding built on it. TypeScript is not
implemented.

## 1. What crosses

**A consumer reaching chdef through the ABI never has to reimplement a
rule this specification states.** That criterion draws the line in both
directions, and it decides what the ABI grows next.

Crossing, because a consumer that writes it holds a second implementation
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

Not crossing, because a consumer writes it whatever chdef does: editing
UI, undo history, save orchestration, and the presentation choices of the
specification index's out-of-scope list. An ABI that carried these would
be deciding for the application.

The grid crosses **as cells**, not as a typed editing API: cells are what
the round-trip guarantee of [editing.md §2](./editing.md#2-round-trip) is
about, and a consumer displaying a definition file needs no column
vocabulary to do it.

## 2. Calling conventions

- **Statuses are `int32_t`.** `CHDEF_OK` is `0`, every failure is
  negative, and `CHDEF_PANIC` is returned rather than letting a panic
  cross `extern "C"`. A call that returns a length instead returns `0`
  where it would have failed.
- **No enumerations cross.** A category — a data type, an issue code, a
  display format — crosses as an ASCII string, so adding one is not an ABI
  break and a caller that does not know it still has something to show.
- **Handles are opaque and tagged.** A handle carries a tag its creating
  call sets and its freeing call clears, so a stale or foreign pointer is
  reported as `CHDEF_ERR_HANDLE` instead of being dereferenced. Freeing
  twice is safe; a null handle is ignored.
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
  readings, and read one reading's displayed value or rendered text.
- **Bits** — a channel's named bits (number, name, memo, and the
  protocol-spec default it carries or its absence), and the bits of a
  decoded frame with the value each holds.
- **Grid** — parse definition bytes into cells, read the header and any
  cell, set a cell, insert / append / remove a row, write the file back.
- **Value notation** — read the text form of a value into the value it
  denotes.
- **Vocabulary** — the canonical column names, and a vocabulary built from
  them that a parse reads its headers with. A column crosses as its
  canonical **name**, not a number, so adding one to the format is not an
  ABI break.
- **Diagnostics** — the count, the numbers and the text of each finding.

A frame's bits are decoded in one pass, not one call per bit: the count
comes from the layout and one call fills the array, so reading every bit
of a frame costs what reading every channel costs.

## 4. Version

`chdef_abi_version()` returns `CHDEF_ABI_VERSION`, which **increments
whenever a symbol is added or changed**. A caller checks that it is
**at least** the value its declarations were written for. Symbols are
added and never withdrawn, so a newer library serves an older caller; the
check catches the reverse, which is a caller asking for symbols that are
not there.

The .NET package carries the native library for every runtime it
supports, so a consumer taking that route cannot pair the two wrongly.
