# ADR-0016: The layout carries the capacity it is measured against

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.3

## Context

ADR-0008 made `capacity` an argument: `check_capacity(capacity)`, on the
reasoning that a `capacity: Option<usize>` field would be "a `None` most
callers pass forever, to serve the few that have a capacity".

A consumer integrating chdef reported the cost of that. Its own `Layout`
wrapper existed for one reason — to hold `max_bytes` beside the chdef
layout — because the two always travel together and only the consumer
could keep them together. The interchange JSON had already conceded the
point: `docs/spec/interchange.md` §1 lists `capacity` among the keys of
a definition set, so the wire format treats it as part of the layout
while the type did not.

The rejected alternative in ADR-0008 was a **parameter on
`build_layout`**, which every caller would have to supply. A field on a
struct the caller already holds is not that: nobody passes anything.

## Decision

- **`ChannelLayout::capacity: Option<usize>`**, with
  `with_capacity(n)` to set it. The struct is `#[non_exhaustive]`
  (ADR-0005), so the field is a non-breaking addition.
- **`check_capacity()` takes no argument** and reads the field, returning
  `None` when there is no capacity — the "without `capacity` there is no
  check" of `docs/spec/layout.md` §5, unchanged in meaning.
- **`Definitions::of` reads the layout's capacity**, so the JSON key
  appears without the caller restating it. `Definitions::with_capacity`
  remains for a capacity that is not the layout's.

ADR-0008's other decisions stand; only the shape of the capacity query
changes.

## Alternatives rejected

- **Keeping the argument form as well**: two ways to ask the same
  question, differing in which one silently ignores the field.
- **A required capacity**: most consumers have no packet limit to state,
  and inventing `usize::MAX` for them would make the check a lie.

## Consequences

- Breaking for callers of `check_capacity(n)`; pre-publish, and the fix
  is `layout.with_capacity(n).check_capacity()`.
- The consumer's wrapper type has nothing left to hold.
- `docs/spec/layout.md` §5 says the layout carries a capacity again,
  which the text had denied since the ADR-0008 cycle.
