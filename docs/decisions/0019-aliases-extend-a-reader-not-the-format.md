# ADR-0019: A column alias extends one reader, not the format

- Status: Superseded by [ADR-0024](./0024-a-vocabulary-is-data-not-a-language.md)
- Date: 2026-08-24
- Release: 0.0.3

## Context

A consumer's definition files word some columns their own way. ADR-0003
made the column vocabulary part of the specification — two canonical
spellings per column plus a fixed alias list — and the golden vectors of
`docs/spec/interchange.md` §3 are what an implementation in another
language conforms to. Letting each consumer widen the vocabulary at
runtime looked, at first, like making the format itself configurable: the
same file would parse differently in two consumers, and there would be
nothing left for the vectors to certify.

The alternative on offer was to put every consumer's wording into the
specification's alias column. That is worse. The format's definition
would accrete every private vocabulary that ever met it, without bound
and without any way to retire one, and every other implementation would
have to carry them all to conform. A spelling used by one team in one
repository is not a fact about the CH CSV format.

What the objection was actually protecting is narrower than the ban it
suggested: that a canonical spelling always means what the specification
says, and that conformance stays a well-defined thing.

## Decision

`ColumnAliases` teaches one reader extra header spellings, under three
rules that keep the format a format:

- **An alias only ever adds.** A cell is matched against the
  specification's spellings first; the taught ones are consulted only for
  a cell nothing canonical claims. Teaching `number` to mean `bytes` does
  nothing.
- **No alias reaches the writer.** A file keeps the header it was read
  with (`docs/spec/format.md` §2), and a file chdef creates uses the
  canonical spellings. A file is never quietly rewritten into, or out of,
  a consumer's private wording.
- **No alias appears in the golden vectors.** Conformance is defined on
  the canonical vocabulary alone, so what a C# or TypeScript
  implementation must do is unchanged by anything a consumer teaches its
  own reader. Aliases are a reader convenience layered on top of a
  conforming reader, not part of what it must be.

The configured reader is a different call — `ChTable::parse_with` beside
`ChTable::parse` — so a file read with a private vocabulary is visibly
read that way at the call site.

## Alternatives rejected

- **Adding each consumer's spelling to the specification** (this ADR's
  first instinct): unbounded accretion into the format's own definition,
  imposed on every implementation.
- **Renaming the header before handing the file over**: makes every load
  a text-manipulation step, or edits files the consumer does not want
  edited.
- **Aliases that also rewrite the header on save**: canonicalising
  someone's file behind their back, and the round-trip guarantee of
  `docs/spec/editing.md` §2 says the header comes back as it was read.
- **An alias that can rebind a canonical spelling**: the point at which
  the format would stop being one.

## Consequences

- `ColumnAliases`, and `parse_with` / `parse_bytes_with` on both tables.
  The typed free functions (`parse_ch_csv`, `load_ch_csv`) stay on the
  canonical vocabulary; a consumer with aliases goes through the table.
- `docs/spec/format.md` §2 states the three rules, so an implementation
  in another language knows aliases are optional and outside the contract.
- A spelling that turns out to be genuinely common can still be promoted
  into the specification's alias column later — that decision stays
  chdef's, and now has a place to be argued from rather than being the
  only option.
