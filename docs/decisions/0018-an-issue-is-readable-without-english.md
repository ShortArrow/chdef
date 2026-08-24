# ADR-0018: An Issue is readable without reading its English

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.3

## Context

`Issue` carried a stable `code` and one English `message`. A consumer
building a Japanese interface reported the consequence: it had
hand-written a sentence for each of the twelve codes it saw, because the
facts inside chdef's sentence — the cell that could not be read, the
value used in its place, which channel the finding is about — existed
only inside the English prose. Anything more specific than "code 12
happened here" meant parsing that prose.

They asked for the values as fields rather than a locale feature, and
they were right to: a translation table inside chdef would serve the
languages chdef anticipates, while fields serve every consumer in every
language, including ones that build a sentence out of their own domain
words rather than chdef's.

Two of these facts are not recoverable any other way:

- **The rejected text.** Parsing replaces it; nothing in `ChannelDef`
  remembers that `bytes` said `99` before it was clamped to `8`.
- **The identity of a rowless finding.** The cross-file checks and
  `encode` carry no row (ADR-0008), so `docs/spec/diagnostics.md` had to
  say "the message names `(number, bit)`" — an instruction to parse
  English, written into the specification.

## Decision

`Issue` gains four fields, and becomes `#[non_exhaustive]` so more can
follow:

- **`found: Option<String>`** — the value chdef could not use, spelled as
  the file spells it. A raw value keeps its cell's notation, so `0x1FF`
  comes back as `0x1FF` and `511` as `511`.
- **`used: Option<String>`** — the value chdef used instead, where it
  substituted one.
- **`channel: Option<u32>` / `bit: Option<u8>`** — which channel, or
  which bit of which channel, the finding is about. Set on every finding
  raised once the identity is known, including the rowless ones.

**`message` stays, and its wording is explicitly not part of the
contract.** It is an English rendering of the same facts, for a log and
for a reader who wants one; a consumer that builds its own sentence uses
the fields and never reads it.

The interchange JSON carries the four fields alongside `code`.

## Alternatives rejected

- **A locale feature inside chdef**: chdef would own translations for
  languages it cannot review, and a consumer whose wording differs from
  chdef's — because its users say "signal" where chdef says "channel" —
  would still be stuck.
- **A generic `params: Vec<(&str, String)>`**: fully general, and the
  consumer must learn a key vocabulary per code, which is parsing
  English with extra steps.
- **Leaving the identity in the message** and telling consumers to read
  it: the specification was already apologising for this.
- **Typed `found` / `used`** (a number where the value is numeric): the
  values go into a sentence, and half of them are the text that failed to
  be a number.

## Consequences

- Breaking: `Issue` is `#[non_exhaustive]`, so it can no longer be
  constructed or matched exhaustively outside chdef. Nothing outside
  chdef should construct one.
- Every construction site inside chdef goes through a small builder, so
  a new field is one method rather than eleven literals to update.
- `docs/spec/diagnostics.md` §2 states the fields and the message's
  status; the footnote telling consumers to read `(number, bit)` out of
  the message is gone.
