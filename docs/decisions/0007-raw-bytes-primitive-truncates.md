# ADR-0007: The raw→bytes primitive truncates; clamping belongs to the physical storey

- Status: Accepted
- Date: 2026-08-21
- Release: 0.0.2

## Context

chbridge's pattern generator writes counters that must **wrap** at the
channel width so they cycle naturally, and it writes raw values directly
— no lsb / offset. chdef only exposed the upper storey,
`value_to_bytes_endian`, which converts a *physical* value with rounding
and a clamp. Unable to climb down to a raw→bytes layer, chbridge kept a
hand-rolled `write_value_endian` — a bypassed abstraction, which is a
discarded one wearing a version number.

## Decision

Split the conversion into two public storeys on `ChannelDef`:

- **`raw_to_bytes_endian(raw, endian)` / `raw_from_bytes_endian(bytes,
  endian)`** — the primitive pair. A raw bit pattern goes to / comes
  from the channel's `byte_count` bytes in the given order. **No
  rounding, no clamp: bits beyond the width are cut.** A caller that
  wants a wrapping counter passes the wrapped raw and the byte cut is
  the modulo; a caller that wants saturation uses the storey above.
- **`value_to_raw` / `value_to_bytes_endian`** — the physical storey,
  unchanged: rounding half away from zero and the width clamp of
  `docs/spec/conversion.md` §2. It now rests on the primitive.

Truncation is the only defensible primitive behaviour: a raw value is
the caller's statement of the exact bits, so "fixing" it would be
policy. Saturation vs wrap-around is exactly the policy split between
the two storeys.

## Alternatives rejected

- **A `wrap` flag on `value_to_raw`**: multiplies the upper storey's
  modes instead of exposing the layer both modes share; the flag would
  also have to answer what wrapping a *physical* value even means.
- **Leaving chbridge's hand-rolled writer in place**: the widths chdef
  learns (3-byte channels, 64-bit) would never reach it.

## Consequences

- `pattern.rs` in chbridge can delegate: it keeps its one line of policy
  (truncate the f64 toward zero, let it wrap) and drops the per-width
  match.
- Frame decode reads raw values through `raw_from_bytes_endian`, so the
  integer path is one implementation.
- The pair round-trips by construction for values within the width.
