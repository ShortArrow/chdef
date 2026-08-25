# ADR-0029: A derived channel states what it covers, and sealing is a call of its own

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.11

## Context

`kind` gained `derived` as a reserved value in
[ADR-0025](./0025-kind-records-who-fills-a-channel.md), left without
meaning until the CRC rules were settled. A consumer supplied them: the
polynomial x¹⁶+x¹²+x⁵+1, an initial value of 0xFFFF, a final XOR of
0xFFFF, and "bits shifted right".

They also said they did not know what makes a CRC unique, which is a fair
thing not to know: "CRC16-CCITT" names at least three different functions.
The four facts above resolve to one catalogued variant — CRC-16/IBM-SDLC,
also called X-25 — confirmed by computing the catalogue check value of
`"123456789"` two independent ways and getting 0x906E both times.

Two questions then had no obvious answer.

**What does a recipe cover?** A frame is commonly sync, length, data, CRC,
and which of those the CRC spans differs by device: the data alone, the
length and the data, or everything before the CRC.

**Where does the value get filled?** ADR-0025 decided `kind` is a mark and
not a behaviour, and `docs/spec/format.md`, `ChannelKind`'s documentation
and a test all say `encode` produces the same bytes whatever `kind` says.
A `derived` channel filled inside `encode` contradicts all three.

## Decision

**The coverage is stated in the file and has no default.** The `derived`
column carries the recipe and the channels it spans:
`crc16/x25 1..7`, or several spans as `2..3,5..7`. A recipe without a
range is Issue `derived_invalid`.

A default of "every channel before this one" would be right for many
devices. It is refused because being right for many is the problem: the
devices it is wrong for get a silently wrong CRC, which surfaces as
hardware discarding frames and saying nothing. Which bytes a frame covers
is a property of a protocol, not of CRC, and a library that guesses it has
taken a decision that was never its own — the same ground on which
`endian` is set by the consumer and never inferred, and on which
`min` / `max` is never applied.

**The range names channels, not bytes.** Inserting a channel renumbers the
ones after it and a channel range follows; a byte range would quietly
start covering the wrong thing, which is the class of failure ADR-0025
exists to remove.

**Sealing is a call of its own.** `ChannelLayout::seal` fills every
derived channel of a frame; `encode` is untouched and stays a pure
function of the layout and the values it is given. ADR-0025's decision
therefore stands intact rather than gaining an exception, the golden
vectors' `E` lines keep meaning what they meant, and a consumer with no
derived channel sees no change at all.

**A recipe is its six numbers, and a name is a shorthand.** Width,
polynomial, initial value, input reflection, output reflection and final
XOR describe every CRC. `crc16/x25` expands to exactly those, and chdef
ships the catalogued variants as names with no standing the numbers lack.
A device whose CRC is in no catalogue writes the numbers — the shape
[ADR-0024](./0024-a-vocabulary-is-data-not-a-language.md) settled, where a
shipped table must never be the only way to say something.

## Alternatives rejected

- **Defaulting the range to everything before the CRC.** Above.
- **A byte range.** It survives no edit of the definition.
- **`encode` filling derived channels.** One call for the consumer and no
  way to forget, at the cost of ADR-0025's central claim and of an
  "except `derived`" that every later `kind` value would argue for. A
  forgotten `seal` is caught by the receiving device and by
  `derived_mismatches`, so the risk it removes is one that announces
  itself.
- **A second `encode_sealed` entry point.** Two paths to keep aligned
  forever, and two sets of golden vectors to run them through.
- **Named variants only.** A consumer whose CRC is uncatalogued would be
  stuck, which is the defect ADR-0024 removed one storey up.

## Consequences

- Issues `derived_invalid`, `derived_unknown_channel` and
  `derived_mismatch` join the vocabulary; the last is the one place chdef
  says a frame is *wrong* rather than merely unusual.
- The canonical CH header gains a `derived` column, appended, so the
  positional first nine are unchanged again.
- The recipe vocabulary grows like every other: enumerable across the
  boundary ([ADR-0026](./0026-a-growing-vocabulary-is-enumerable.md)), so
  a consumer can ask which recipes this chdef knows.
