# ADR-0006: min / max are physical bounds with a raw escape, carried but never applied silently

- Status: Accepted
- Date: 2026-08-21
- Release: 0.0.2

## Context

The `min` / `max` columns were parked as uninterpreted strings. Deciding
their meaning raised three questions at once.

**Units.** A GUI slider bound or a plausibility limit is naturally a
physical value (−40–120 °C), but the sibling column `default` is a raw
value, and protocol constants are stated raw. The consumer confirmed both
exist in real files.

**Application.** Clamping to the range inside `value_to_raw` / encode
would be convenient for a GUI — and would silently distort any consumer
that records what it was given, with no way to notice. Whether an
out-of-range value should clamp, error, or pass through is policy, and
policy differs per consumer.

**Staleness.** `lsb` and `offset` are editable at runtime (public
fields). A raw bound converted to a physical number at parse time would
go stale the moment `lsb` changes — the same staleness class as
`total_bytes`, which was a stored sum that an edited `byte_count`
silently invalidated.

## Decision

- A `min` / `max` cell is **physical by default; the `0x` prefix means a
  raw bit pattern**, the same escape `default` already uses. The two
  notations live in `Bound::Physical(f64)` / `Bound::Raw(u64)`, kept as
  written.
- A raw bound is resolved **at query time** with the channel's current
  `lsb` / `offset` (`min_value` / `max_value`), so runtime edits move it
  and nothing stales.
- **No conversion applies the range.** The explicit applications are
  `range_contains` (is a value inside?) and `clamp_to_range` (force it
  inside) — named for what they do rather than a generic `apply`.
- Parse diagnoses without discarding: `min_invalid` / `max_invalid`
  (unreadable → unspecified), `raw_out_of_range` (reused; low bits
  kept), `min_max_swapped` (both kept; the range matches nothing).
- The same staleness fix applies to the layout: `total_bytes` became a
  computed method, removing the stored value that runtime edits could
  invalidate, instead of adding an `update()` callers could forget.

The whole-layout `endian` field added alongside is not a decision of
this ADR — it implements `docs/spec/layout.md` §2 as specified.

## Alternatives rejected

- **Raw-only bounds** (naming symmetry with `値(デフォルト)`): the
  dominant use is display limits in physical units; forcing raw would
  push the lsb arithmetic onto every consumer.
- **Resolving raw bounds to f64 at parse time**: stales on runtime `lsb`
  edits and loses the notation the writer must reproduce.
- **Clamping inside encode / `value_to_raw`**: bakes one consumer's
  policy into the mechanism and silently rewrites recorded data. The
  width clamp stays — a raw value must fit the wire; that is
  representability, not policy.
- **An `update()` / recompute ritual for stored derived values**: a call
  sites can forget; a value that is never stored cannot stale.

## Consequences

- `ChannelDef` gains `min` / `max` (`Option<Bound>`) — non-breaking under
  ADR-0005 — plus `min_value` / `max_value` / `range_contains` /
  `clamp_to_range`. `Bound` is exported and closed (two notations are
  the whole grammar).
- `ChannelLayout::total_bytes` is a method; the field is gone.
- The Issue vocabulary grows to 22 codes (19 emitted).
- Frame encode / decode (`docs/spec/conversion.md` §5–6) stays free to
  compose these queries when a consumer asks it to.
