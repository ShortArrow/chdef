# ADR-0024: A column vocabulary is data, and the canonical name is the identity

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.7
- Supersedes [ADR-0019](./0019-aliases-extend-a-reader-not-the-format.md),
  whose decision was that "a column alias extends one reader, not the
  format". The constraint it protected — that a canonical spelling always
  denotes the column the specification says — stands and is restated
  below; the mechanism it chose does not.

## Context

Columns were identified against a table that read, per column:

```rust
ChColumn::Number => &["number", "番号", "no", "ch", "chnumber"],
ChColumn::Name   => &["name", "メッセージ名称", "signalname", "信号名称"],
```

with `HeaderLanguage::En` and `HeaderLanguage::Ja` selecting index `0` or
`1` for writing, and `ColumnAliases` letting a caller add read-only
spellings on top.

Three things are wrong with this, and they compound.

**The number of languages is encoded in an array position.** A third
language is not data to add; it is an enum variant, a new index, and a
new invariant that every row of the table must hold. What looks like
internationalisation is two special cases with a positional convention
holding them together.

**The tail mixes categories.** `信号名称` and `signalname` sit in the same
undifferentiated list, so nothing in the type says which spellings belong
to which vocabulary, and nothing could.

**The behaviour is asymmetric even where the mechanism is not.** A
Japanese file parsed with no configuration; a German file needed
`ColumnAliases`, and could still never be *written*, because ADR-0019
decided no alias reaches the writer. A consumer outside the two
privileged languages got a second-class reader and no writer at all.

Underneath all three is a responsibility in the wrong layer. What a
column *is*, and what each of its cells means, is chdef's domain. Which
string in some file's header denotes that column is an adapter concern,
and adapters belong to the application, not to the thing being adapted.

## Decision

**A column has one canonical name, and everything else is a vocabulary
the caller supplies.**

- **The canonical name is the identity.** Lower-case ASCII —
  `ChColumn::name()` takes no language and returns it. It is what the
  specification, the JSON of `interchange.md`, and every chdef API mean by
  the column. `ChColumn::canonical()` and `ChColumn::variants()` replace
  the positional `spellings()` table, so no convention connects an index
  to a language.
- **`ColumnVocabulary` replaces `ColumnAliases` and `HeaderLanguage`.** It
  is a value: spellings to columns for reading, and one spelling per
  column for writing. **The first spelling taught for a column is the one
  written**, so a vocabulary that reads a header can also write it and no
  separate setter exists.
- **`ColumnVocabulary::japanese()` is one such value**, shipped because
  the format was extracted from files that use those spellings. It has no
  standing a caller-built vocabulary lacks, and it is **not applied unless
  asked for**: reading with no vocabulary recognises the canonical names
  and their English variants alone.
- **A vocabulary reaches the writer**, reversing ADR-0019 on that point. A
  vocabulary that could be read but never written is what made a
  non-Japanese consumer second-class, and the round-trip guarantee already
  prevents the risk that rule was aimed at: a file chdef *reads* keeps its
  own header regardless.

What ADR-0019 protected is kept by two rules, now stated in
`docs/spec/format.md` §2:

- **A vocabulary only adds.** Canonical names and variants are matched
  first, so teaching `number` to mean something else does nothing.
- **No vocabulary appears in the golden vectors.** Conformance is defined
  on canonical names alone, so an implementation in another language owes
  no vocabulary at all.

## Alternatives rejected

- **Keeping Japanese in the default vocabulary.** Compatible, and it
  leaves the defect the change is for: the mechanism would be general
  while the behaviour still privileged one language, so a Japanese file
  parsed out of the box and a German one did not.
- **Adding `HeaderLanguage::De`, `::Zh`, …** — the same design, one
  release later, with the language list owned by whoever happens to
  maintain chdef rather than by whoever has the files.
- **Removing the Japanese spellings from the crate entirely.** Every
  consumer of the format as it exists would then carry the same table, and
  they would drift.
- **A trait for callers to implement.** A vocabulary has no behaviour
  worth dispatching on; it is a lookup table, and a table is easier to
  build, compose, and print than an impl.

## Consequences

- Reading a Japanese-headed file needs `ColumnVocabulary::japanese()`
  passed explicitly. This is a breaking change for every current consumer,
  and it is one line at each call site.
- `HeaderLanguage` is gone, and `ChColumn::name(lang)` is
  `ChColumn::name()`.
- The vocabulary crosses the C ABI and the .NET binding, because
  [ADR-0023](./0023-the-abi-carries-every-rule.md) says every rule a
  consumer would otherwise reimplement crosses, and "which spelling means
  which column" is exactly such a rule.
- `docs/spec/format.md` §2 gained the Japanese vocabulary as a table,
  where it is visibly one vocabulary rather than half the mechanism.
