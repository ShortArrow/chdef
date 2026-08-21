# ADR-0011: One notation-carrying Value type feeds bounds, forms, and encode

- Status: Accepted
- Date: 2026-08-21
- Release: 0.0.2

## Context

A consumer integrating chdef (sensord) reported reinventing the same
notation rule at its input boundary — "`0x` is a raw value, anything
else is physical" — as a `Given::parse` of its own, and asked for frame
encode in the crate so its Rust and C# implementations stop maintaining
the assembly rules twice. The rule they reinvented already existed in
chdef twice over: as the `min` / `max` bound type (`Bound::Physical` /
`Bound::Raw`, ADR-0006) and as the `default` cell's `0x` escape. And the
encode specification (`docs/spec/conversion.md` §5) defines its input as
exactly this pair: "a physical value, or a raw value (`0x`), per
channel".

Three appearances of one concept, one of them named for just one of its
uses.

## Decision

- **`Bound` is renamed `Value`** — a number carrying its notation. The
  CSV columns it feeds are literally named 値 (`値(最小)`, `値(デフォルト)`):
  the domain word was already "value". `min` / `max` keep their
  ADR-0006 semantics unchanged.
- **`Value::parse` is public**: the notation rule for consumer input
  (form fields, cells), trimming, rejecting non-finite numbers. The
  `min` / `max` cell reader now builds on it, adding only the width
  check.
- **`ChannelLayout::encode(&[(u32, Value)]) -> Parsed<Vec<u8>>`**
  implements §5: named channels take their value (physical → §2
  rounding and clamp; raw → §3 low bits), every other channel takes its
  §4 default — `ChannelLayout::channel_default` exposes that merge. The
  last entry for a number wins, map-style.
- **Encode reports what it cannot place instead of dropping it**:
  `encode_unknown_channel` (a number the layout does not have) and
  `encode_value_invalid` (NaN / infinite physical; the default is
  used). Silent loss of caller data would be a hidden failure.

## Alternatives rejected

- **A separate `Given` enum for encode input**: the same two variants
  under a second name, drifting apart from `Bound` one field at a time.
- **Keeping the name `Bound`** for the unified type: an encode input is
  not a bound; the name would mislead at its most common call site.
- **`encode` returning bare `Vec<u8>`**: unknown numbers and NaN would
  vanish silently — the error-shape discipline of the diagnostics spec
  applies to operations too.
- **Bit-level encode input** (set one BF bit): composable today from
  `channel_default` and `bit_of` by the caller; a dedicated input form
  can come when a consumer shows the need.

## Consequences

- Breaking rename pre-publish; chbridge re-exports `Bound` and needs a
  one-word change at its next revision bump.
- sensord's `Given::parse` and both of its frame codecs (Rust and C#)
  can delegate; the C# side gains a fixed reference implementation to
  conform to.
- The diagnostics vocabulary grows to 24 codes, all emitted.
