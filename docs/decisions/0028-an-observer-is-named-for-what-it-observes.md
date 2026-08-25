# ADR-0028: An observer is named for the state it observes

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.10

## Context

A reviewer asked whether this crate distinguishes **state** — what is true
now and changes over time — from **property** — what is tied to the thing
and does not. "Not broken" is a state; "cannot be broken" is a design.

The design mostly held. `kind` is a property of a channel and lives in the
definition, while the count a `counter` is at is state belonging to the
line that sends the frames, which is why chdef never advances one
(ADR-0025). A width clamps, so a frame cannot be malformed by construction;
a range does not, so chdef only observes it. Handle tags refuse a handle of
the wrong kind by design, and the specification says plainly that a freed
handle is undefined rather than claiming a guarantee it cannot keep.

The naming did not hold. Methods that observe and change nothing were
named for the act of observing:

```rust
range_contains(v) -> bool          // the property, asked
fits_width(v)     -> bool          // the property, asked
check_values(&[]) -> Vec<Issue>    // an operation, returning a state
check_capacity()  -> Vec<Issue>    // an operation, returning a state
```

A caller reading `check_values` learns that something is checked. A caller
reading `values_out_of_range` learns what comes back.

## Decision

**A method that changes nothing is named for the state or property it
reports, not for the act of reporting it.** A verb names an operation that
does something: `encode`, `decode`, `render`, `clamp_to_range`,
`insert_channel`.

Renamed accordingly:

| was | is |
|---|---|
| `check_capacity()` | `limits_exceeded()` |
| `check_values(…)` | `values_out_of_range(…)` |
| `check_readings(…)` | `readings_out_of_range(…)` |
| `ChTable::range_issues()` | `ChTable::defaults_out_of_range()` |

with `chdef_layout_check_capacity` becoming `chdef_layout_limits_exceeded`
and `Definitions.CheckCapacity` becoming `Definitions.LimitsExceeded`
across the ABI and the binding.

`limits_exceeded` also corrects a name that had outlived its subject:
since 0.0.8 it answers for two limits, not just the byte capacity.

## Alternatives rejected

- **Renaming only the unreleased ones.** It was the cautious option and it
  would have frozen the inconsistency into the released surface, where the
  next reader learns the wrong rule from `check_capacity` and copies it.
  The 0.0.x line allows a break and announces it.
- **Keeping `check_*` and renaming the predicates to match.**
  `check_range_contains` is worse for every caller, and the predicates were
  the half that was already right.
- **Treating this as a matter of taste.** The names differ in what they
  tell the caller about the return value, which is not taste.

## Consequences

- Every consumer calling `check_capacity` / `CheckCapacity` edits one line
  per call site. Announced under `### Breaking`.
- The rule decides the next addition without another discussion, and it is
  why the range asks arrived named as they are rather than as
  `check_ranges`.
- Records that name the old method — [ADR-0008](./0008-layout-checks-have-no-rows.md),
  [ADR-0016](./0016-the-layout-carries-its-capacity.md), and the CHANGELOG
  entries — are left as written. They were true when written, and a record
  that follows later revisions of its subject is not a record.
