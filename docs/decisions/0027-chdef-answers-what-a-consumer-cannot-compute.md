# ADR-0027: chdef answers about a declared property only where a consumer cannot compute the answer

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.10

## Context

A definition declares properties of a channel that chdef deliberately does
not enforce: the `min` / `max` range, and since
[ADR-0025](./0025-kind-records-who-fills-a-channel.md) the `kind`. Nothing
applied them, and nothing answered questions about them beyond the
per-channel `range_contains`.

Two asks arrived together: a consumer wanted to know, before sending and
after receiving, which values fall outside their declared range, and
wanted to colour a grid cell whose `default` violates its own row.

Adding the range asks exposed an inconsistency in ADR-0025, which declined
to report a value supplied for a `const` channel on the ground that "what a
caller may send is the caller's to decide". That reasoning was about
`encode` reporting **automatically**. It says nothing about a caller
*asking*, and the range asks are exactly that — so the same argument now
appeared to permit one and forbid the other.

## Decision

**chdef answers a question about a declared property when the answer
cannot be computed from what already crossed the boundary, and not
otherwise.**

- **The range is answered.** `min` / `max` reach a consumer *in the
  notation their cells used* — `"0x1F"`, `"100"`, or empty. Deciding
  whether a value is inside means reading the `0x` prefix, resolving it
  through the channel's current `lsb` and `offset`, sign-extending for
  `SI`, and handling an unspecified or swapped side. That is a chain of
  chdef's rules, and a consumer reimplementing it is the divergence
  [ADR-0023](./0023-the-abi-carries-every-rule.md) exists to prevent. The
  asks are `values_out_of_range`, `readings_out_of_range` and
  `ChTable::defaults_out_of_range`.
- **The kind is not answered.** `kind` crosses as a finished string —
  `"const"`. Whether a caller supplied a value for such a channel is a
  comparison between two things the caller already holds, and it is one
  line at the call site. Adding a chdef call for it would put a rule in
  the library that is not chdef's to hold.
- **Answering is never enforcing, and never a mode.** None of the asks
  changes what `encode` or `decode` do, and none is remembered on the
  layout. Whether the answer is wanted is a question of the moment.

This replaces the reason ADR-0025 gave. That record's decision — that
`kind` is a mark chdef does not act on — stands; the ground for refusing a
`const` check is the criterion above, not an argument about whose business
the caller's values are.

## Alternatives rejected

- **An ask for `const` too, for symmetry.** Symmetry between a rule chdef
  owns and a string comparison is a false one, and the surface would grow
  with every property `kind` gains.
- **Applying the range in `encode`.** Declined earlier and still declined:
  a value outside a declared range is written exactly as given, so nothing
  is hidden and there is nothing to confess. A caller sending outside the
  range during calibration is not making a mistake.
- **Resolving `min` / `max` into numbers at the boundary** so a consumer
  could compare them itself. It would move the resolution rules across
  intact, but they resolve against the channel's *current* `lsb` and
  `offset` (conversion.md §8) — an edited `lsb` moves the bound, and a
  number handed over once would be stale.

## Consequences

- Issue `value_out_of_range` joins the vocabulary, with `used` naming the
  bound that was crossed.
- `docs/spec/conversion.md` §8 states the three asks and that a range is
  observed rather than enforced.
- The criterion applies to the next declared property as it did to these
  two. `derived` will need it when the CRC rules land: computing a CRC is
  chdef's rule, so it will be answered.
