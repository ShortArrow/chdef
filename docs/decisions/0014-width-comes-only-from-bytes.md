# ADR-0014: The width comes only from `bytes`, and `DataType` stops carrying one

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.2

## Context

`docs/spec/layout.md` §6 has said since the first release: "The width is
`bytes` (1–8). `type` carries no width, only the interpretation (`UI` /
`SI` / `BF`)." The `DataType` enum, inherited unexamined from the
extraction, contradicted that at the type level — `UI8`, `UI16`, `UI32`,
`SI8`, `SI16`, `SI32` each bake a width into the interpretation — and a
`resolve(parsed, byte_count)` function existed only to pick one of those
variants from the real width.

`resolve` cannot represent 3, 5, 6 or 7 bytes, and collapses 8 to the
16-bit variant. Meanwhile `bits()`, `value_to_raw`, `raw_to_bytes_endian`
and `raw_from_bytes_endian` all measured the channel with `byte_count`.
Only `raw_to_value_endian` measured it with `DataType` — so it read two
bytes of a channel every other function treated as three, five, six,
seven or eight bytes wide.

Measured before this change, from definitions that load with no Issue:

- 3-byte `SI`: `−100 000` encoded, then decoded as **`+31 072`** — a
  silent sign flip. `−8 388 608`, the exact 24-bit minimum, decoded as
  `0`.
- 8-byte `UI`: `ChannelLayout::decode` returned `raw` and `value`
  disagreeing inside one `Decoded` (`0x0102030405060708` and `513`).
- 3-byte `BF` read big-endian was shifted eight bits, because the `BF`
  arms zero-padded to four bytes at the tail regardless of byte order.

Five of the eight legal widths were wrong, and nothing reported it.

## Decision

- **`DataType` is `UI` / `SI` / `BF`** — the interpretation, nothing
  else. `byte_count()` and `resolve()` are gone; `as_str()` (public, as a
  consumer asked) and `Display` give the two-letter tag.
- **`ChannelDef::width()` is the single authority**: `byte_count` held to
  1–8, the same range `docs/spec/format.md` §3 clamps the column to.
  `bits()`, both raw↔bytes primitives, `total_bytes()`, `positions()`,
  `channel_offset()`, `channel_end()`, `encode` and `decode` all measure
  the channel with it. A `byte_count` outside the range is read as the
  nearest legal width instead of panicking — `bits()` can no longer be
  zero, which also removes a subtraction overflow in `raw_to_value_u64`.
- **`raw_to_value_endian` is composed**, not reimplemented:
  `raw_to_value_u64(raw_from_bytes_endian(bytes, endian))`. One byte
  reader, one integer conversion, no third path to disagree with them.
- **`raw_to_value_u64` sign-extends at 64 bits too.** Its guard excluded
  the 64-bit case, so an 8-byte `SI` channel read `−1` as `1.8e19`, which
  also poisoned `min_value` / `max_value` and every range query built on
  them.
- **`value_to_raw` clamps in integer space.** A bound of the form
  `2^n − 1` is not representable in f64 beyond 53 bits; rounding it up to
  `2^n` and then masking erased the value, so a 7-byte unsigned channel
  clamped to `0` instead of its maximum.

## Alternatives rejected

- **Making `raw_to_value_endian` match on more variants** (`UI24`,
  `UI40`, …): 24 variants to state a number the channel already carries,
  and the contradiction with layout.md §6 would remain.
- **Rejecting an out-of-range `byte_count` in `ChannelDef::new`**: a
  fallible constructor for a case the CSV path already clamps, and every
  caller would carry the `Result` for it.
- **Keeping `resolve` for compatibility**: pre-publish, so there is no
  compatibility to keep — only a defect to keep.

## Consequences

- Breaking for anyone matching `DataType`, and for chbridge, whose facade
  re-exports it. Pre-publish, and the fix is mechanical (`UI32` → `UI`).
- `docs/spec/conversion.md` no longer excludes 64-bit physical decode; the
  f64 precision caveat of §1 is the only remaining limit.
- The defects above were found by tests derived from the specification's
  own wording over all eight legal widths, after tests written alongside
  the implementation had exercised only widths 1, 2 and 4 for six
  releases. The 7-byte clamp defect was found by those tests alone — no
  reviewer had reported it.
