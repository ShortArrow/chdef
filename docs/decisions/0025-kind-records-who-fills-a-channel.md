# ADR-0025: `kind` records who fills a channel, and chdef does not fill it

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.8
- The ground this record gives for declining a `const` check is
  replaced by [ADR-0027](./0027-chdef-answers-what-a-consumer-cannot-compute.md);
  the decision that `kind` is a mark chdef does not act on stands.

## Context

A consumer reported that "which channel carries the frame number" was
written in six places in its own code: a `FrameNoChannel = 2` constant in
C#, a `FRAME_NO_CH = 2` constant in TypeScript, and — worst — a
`ch.At < 4` byte-position comparison that does not mention the channel
number at all and so breaks on a width change as well as a renumber.

The same day, they inserted a sync word as channel 1 and every channel
number after it moved by one. `ChTable::insert_channel_renumbering`
returns `Renumbered { moved }` precisely because external references break
at that moment; every one of those six places went silently wrong, and
`moved` cannot repair a constant it has never heard of.

The fact "channel 2 is the frame counter" belongs to the definition, and a
cell in the row cannot go out of step with the row.

The request originally asked for `const` / `counter` / `derived` as one
column with chdef filling each. Those three are not one thing:

- `const` is already expressible — a `default` that nobody overrides.
- `derived` (a CRC) is computable inside `encode` from the rest of the
  frame, with no state.
- `counter` needs the previous frame, so `encode`, which takes `&self` and
  holds nothing, cannot produce it.

## Decision

**`kind` is a mark, not a behaviour.** chdef reads the column, carries it
on `ChannelDef`, exposes it across the ABI and the binding, writes it
back, and reports a value it cannot read. `encode` produces the same bytes
whatever `kind` says.

- The values are `plain` (the default, and what an empty cell means),
  `const`, and `counter`. An unrecognised value is read as `plain` with
  Issue `kind_assumed`, the discipline `type_assumed` already follows.
- **chdef does not advance a `counter`.** A counter belongs to the line
  that sends the frames, and one definition may be shared by several lines
  with a running number each; a counter held by the layout would have the
  two lines eating each other's numbers. The caller supplies the value and
  wraps it, which the channel width already defines.
- **Overriding a `const` channel is not an Issue.** An Issue would change
  no bytes, and `raw_display_with_lsb` is precedent for chdef judging a
  combination the caller chose. It was declined anyway: what a caller may
  send is the caller's to decide, and one exception invites the same
  argument for every value `kind` gains later.
- **`derived` is not in this release.** A value chdef carries but cannot
  act on is a promise with no content. Categories cross the ABI as strings
  (ADR-0021) so the set can grow: a file written with `derived` today
  loads as `plain` with an Issue, and starts meaning something on the day
  the CRC rules are specified.
- **The column is appended to the canonical order**, not placed beside
  `type` where it reads best. The first nine columns are frozen by the
  positional form of `format.md` §2.

## Alternatives rejected

- **chdef advances the counter, with the state in a value the caller
  passes.** Not hidden state, and the wrap rule would have one home. It
  still puts the counter in the wrong place: the state belongs to a line,
  the layout does not know how many lines share it, and the rule it would
  centralise — advance by one, wrap at the channel width — is one
  expression at the call site.
- **`const` / `counter` / `derived` in one release.** `derived` drags the
  polynomial, the width, the initial value, the reflections and the
  covered range in with it, and turns "add a column" into "specify CRC".
- **Leaving it to a consumer convention.** That is the six places.

## Consequences

- The canonical CH header is 17 columns. The positional first nine are
  unchanged, so the 9- and 10-column forms read exactly as before.
- `ChannelKind` is `#[non_exhaustive]`: the set will gain `derived`, and a
  caller matching on it needs a catch-all arm, as `DataType` already
  requires.
- The consumer's six references become one cell. `Renumbered { moved }`
  still exists for references chdef cannot see, and now has less to
  repair.
