# ADR-0026: A vocabulary that grows is enumerable across the boundary

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.8

## Context

[ADR-0021](./0021-the-c-abi-is-a-codec-not-the-crate.md) decided that no
enumeration crosses the ABI: an Issue code, a data type, a display format
each cross as a stable ASCII string, "because both vocabularies are
documented as growing". That is right, and it leaves a consumer holding a
string with nothing to check it against.

A consumer reported the cost. `Issue.Code` reaches C# as a `string`, so a
misspelling compiles and silently falls through to the English `Message` —
the one field whose wording `docs/spec/diagnostics.md` explicitly does not
contract. Their own Japanese message table is keyed by those strings, and
when `type_assumed` was added the table did not know: a test expectation
moved from one to two and that was the only signal.

The complaint is not that the codes are strings. It is that a caller
cannot ask what the codes *are*, so nothing on their side can be checked
for completeness at build time.

0.0.7 already solved this shape once without naming it. Columns cross as
canonical names rather than numbers, and `chdef_column_count` /
`chdef_column_name` let a caller enumerate them; the .NET binding's
`ChColumn` enum is checked against that list in a test rather than being a
second list.

## Decision

**A vocabulary that crosses as strings is enumerable across the same
boundary.** Where chdef says "new values may appear", it must also say
what the values are today.

- `IssueCode::all()` returns every code, and
  `chdef_issue_code_count` / `chdef_issue_code_name` report them across
  the ABI, in the shape `chdef_column_count` / `chdef_column_name`
  established.
- The .NET binding exposes the codes as **constants, not an enum**, beside
  an `IssueCode.All`. An enum would have to be widened every time chdef
  adds a code, and an unknown code arriving from a newer native library
  would have nowhere to land; a constant catches the misspelling that was
  the actual complaint, and an unrecognised code still travels as the
  string it is.
- A test asserts the constants are exactly what the ABI enumerates, in
  order — so the binding holds no second list, only a checked mirror.

`Issue.Code` stays a `string`. Making it a type would re-decide ADR-0021
for no gain: the enumeration is what was missing.

## Alternatives rejected

- **A C# `enum` for the codes.** What the consumer asked for first. It
  turns every added code into a breaking change for them and has no answer
  for a code it has not heard of.
- **Shipping localised message text.** Also asked for, and declined:
  [ADR-0018](./0018-an-issue-is-readable-without-english.md) split `code`,
  `found` and `used` out of `message` exactly so a consumer writes its own
  sentence. The drift they hit is fixed by being able to enumerate, not by
  chdef owning their wording.
- **Documenting the code list in prose only.** `docs/spec/diagnostics.md`
  already does, and prose is not something a test can iterate.

## Consequences

- Adding an Issue code stays additive for a consumer that checks
  completeness against `all()`; it becomes a visible, one-line failure
  instead of a silent gap.
- The same obligation now applies to any future vocabulary chdef adds. The
  `kind` values of [ADR-0025](./0025-kind-records-who-fills-a-channel.md)
  are the next candidate, and are deliberately left out until a second
  caller needs them enumerated rather than read.
