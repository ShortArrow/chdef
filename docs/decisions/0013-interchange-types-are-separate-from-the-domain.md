# ADR-0013: The JSON shape is its own type, and the golden vectors are the cross-language contract

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.2

## Context

Two consumers reimplement the same rules chdef already owns. One writes
its layout JSON by hand to feed a browser UI; another keeps a second
implementation of the CSV reading and the frame codec in C#, pinned to
the Rust one by a private golden file it maintains itself. Both asked
for the crate to carry what they had rebuilt.

`docs/spec/interchange.md` already fixed both shapes — the JSON keys and
a `vectors.txt` grammar — and marked them unimplemented. The open
question was how to produce the JSON: derive `Serialize` on the domain
types, or build a separate value.

The shapes differ from the domain types in ways that are not accidents.
The JSON uses short keys (`n`, `at`), states the position that
`ChannelLayout` computes rather than stores, spells the format `DEC` /
`HEX` where the type is an enum, renders `min` / `max` in their source
notation as strings, and names the category (`UI`) with the width
carried separately in `bytes`.

## Decision

- **The interchange types are their own** (`interchange::Definitions`,
  `ChannelJson`, `BitFieldJson`, `IssueJson`, `Readings`, `TableJson`),
  behind the `serde` feature. A field added to `ChannelDef` does not
  move the JSON, and the JSON's spellings do not constrain the domain.
- **chdef builds the value; the consumer serialises it.** No serializer
  is a dependency of the crate, so the caller keeps whichever it already
  uses and pays for nothing else.
- **`capacity` is absent, not null, when none applies**, matching the
  specification's "present only when one was passed".
- **A physical value that is not a number serialises as `null`** — JSON
  has no other way to carry it, and the specification says so.
- **The golden vectors ship inside the package**
  (`crates/chdef/vectors/<name>/`), so a consumer of the published crate
  receives the contract, not only someone who clones the repository. An
  integration test walks every set and fails on the first mismatch,
  naming the vector file and line.
- **A vector set's definitions must load without Issues.** A contract
  file whose own CSV is questionable would be arguing two things at
  once.

## Alternatives rejected

- **`#[derive(Serialize)]` on `ChannelDef` / `ChannelLayout`**: welds
  the frozen wire format to the type that is meant to keep growing —
  every new field would silently enter the JSON, and the short keys
  would have to become the domain's own names.
- **A `to_json_string` on the crate**: takes the serializer choice, the
  allocation and the error type away from the caller for one line of
  convenience.
- **Generating the vectors from the implementation**: a golden file
  computed by the code it checks proves nothing. The values in
  `vectors/basic/vectors.txt` were derived from the specification by
  hand, and the harness's own detection was confirmed by mutating a
  vector and watching it fail.

## Consequences

- A consumer's hand-written layout JSON can be deleted, and a C#
  implementation can claim chdef conformance by running the same files.
- The JSON shape is now frozen in the sense of Hyrum's law: adding a key
  stays compatible, renaming or removing one does not.
- `Value` gained a `Display` that writes exactly what `Value::parse`
  reads, so the notation has one renderer shared by the CSV writer and
  the JSON.
- TypeScript type generation, which the specification also mentions, is
  still not implemented.
