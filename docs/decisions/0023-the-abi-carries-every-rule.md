# ADR-0023: The ABI carries every rule a consumer would otherwise reimplement

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.5
- Supersedes the "The C ABI is a codec, not the crate" framing of
  [ADR-0021](./0021-the-c-abi-is-a-codec-not-the-crate.md) — the ABI is
  the crate's rules, not a subset chosen by convenience. That record's
  decision that no enumeration crosses stands, and is restated in
  `docs/spec/abi.md` §2.

## Context

[ADR-0022](./0022-the-dotnet-binding-ships-here.md) shipped a C ABI and a
.NET binding on the argument that "the duplication is removed the day the
C# project calls the ABI, not the day the ABI exists", and recorded as its
consequence that the consumer's `ChdefCsv` and `ChdefCodec` "can be
deleted rather than pinned".

Neither can. A consumer that took the binding as far as it goes reports
that the ABI carries the layout, encode and decode, and carries neither
the named bits of a channel nor the file's cells. So a GUI that draws bit
fields as checkboxes still reads the BF CSV itself, an editor still holds
its own CSV reader and writer, and the notation "`0x` means raw" is
written a second time on the C# side. Their conclusion is that adopting
the binding at all would leave *two* CSV readers side by side, which is
worse than one, and that the migration is therefore on hold.

That is the same argument ADR-0022 made, arriving one storey lower. An
ABI that carries most of the rules leaves the consumer holding the rest,
and the rest is where the four divergences the golden vectors caught
actually lived.

The general failure in both records is that the boundary was drawn by
*what was convenient to expose* — first the header alone, then the numeric
conversions — rather than by a criterion. Without one, every consumer
request is relitigated from scratch and the answer keeps being "half".

## Decision

**The ABI carries everything a consumer would otherwise reimplement, and
stops at what a consumer writes anyway.**

Concretely, a rule stated in `docs/spec/` crosses the ABI. The wire rules,
the file rules, the value notation, the diagnostics: each has one home,
and a consumer writing it in another language creates a second home. What
does not cross is what chdef has no opinion about — editing UI, undo
history, save orchestration, presentation — because carrying those would
be deciding for the application rather than serving it.

Applied to the surface as of this decision, four groups are added:
the named bits of a channel and of a decoded frame; the grid; the value
notation; and a reading's displayed value and rendered text.

Three subsidiary decisions follow from the shape of that surface:

- **The grid crosses as cells, not as a typed editing API.** Cells are
  what the round-trip guarantee is about, and a consumer displaying a
  definition file needs no column vocabulary to do it. The typed
  operations — inserting a channel, renumbering — are not exposed until a
  consumer shows they are needed, by the same rule that produced this
  record.
- **A frame's bits decode in one pass.** The requested shape was a call
  per bit; that re-reads the frame once per bit, and an ABI whose cost
  grows with the number of things asked for gets worked around. The count
  comes from the layout and one call fills the array, exactly as channel
  readings already work.
- **A row is inserted empty and filled with cell writes.** Passing an
  array of strings across the boundary buys nothing over the `set_cell`
  that has to exist anyway, and `set_cell` already pads a short row.

**`CHDEF_ABI_VERSION` increments on every added or changed symbol, and a
caller checks that it is at least what its declarations need.** The
previous wording said a caller checks the version "it was written for",
which reads as equality and would break a correct caller on every
addition. Symbols are added and never withdrawn, so the check that matters
is the one-sided one.

## Alternatives rejected

- **Bits now, the grid later.** It is what the consumer would have
  accepted, and it is the shape of mistake this record exists to stop:
  it leaves the CSV reader duplicated, which was the larger of the two
  duplications.
- **Exposing `ChTable` / `BfTable` instead of `Grid`.** The column
  vocabulary would then cross the boundary as a third spelling of the
  same enumeration, and the consumer asked for cells.
- **Letting the consumer keep the value notation.** Three lines of C#, and
  three lines that stop matching `format.md` §3 the moment it gains a
  form. Cheap to write and impossible to keep.
- **A criterion of "expose what is asked for".** It is what produced two
  records of half a boundary. Demand decides *when* a rule crosses, not
  whether it is chdef's rule.

## Consequences

- The ABI roughly doubles, and every addition is frozen on arrival
  (ADR-0005) from three directions now — the header, the C# declarations,
  and the golden vectors.
- The vectors' `F` and `P` lines, previously contracted against the crate
  alone because the ABI had no bits, now run on all three paths. Every
  line of every set is checked everywhere.
- `docs/spec/abi.md` exists to hold the criterion above, so the next
  request is answered by reading it rather than by relitigating the
  boundary.
- The consumer's `ChdefCsv` and `ChdefCodec` can be deleted — the claim
  ADR-0022 made prematurely.
