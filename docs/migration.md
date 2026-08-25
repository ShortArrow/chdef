# Migration

🌐 **English** | [日本語](./migration.jp.md)

What a consumer has to change between 0.0.x releases, assembled in one
place. [The changelog](../CHANGELOG.md) records each release on its own;
this page is for someone jumping several at once.

The 0.0.x line treats each bump as MAJOR-equivalent, so a break is
allowed. Every one is listed here.

## At a glance

| From → to | What you must edit |
|---|---|
| 0.0.5 → 0.0.6 | nothing (0.0.6 never reached a registry) |
| 0.0.6 → 0.0.7 | **pass a vocabulary** to read Japanese headers |
| 0.0.7 → 0.0.8 | `check_capacity()` answers with a list; `Channel.BitCount` is `Channel.Bits` |
| 0.0.8 → 0.0.9 | nothing, but a new Issue appears on traffic that clamps |
| 0.0.9 → 0.0.10 | `check_capacity` is `limits_exceeded` |
| 0.0.10 → 0.0.11 | `Channel` gained `Derived` (positional deconstruction) |
| 0.0.11 → 0.0.12 | nothing |
| 0.0.12 → 0.0.13 | nothing |
| 0.0.13 → 0.0.14 | nothing |
| 0.0.14 → 0.0.15 | nothing; a JavaScript binding is new on npm |

`CHDEF_ABI_VERSION` went 2 → 3 → 4 → 5 → 6 across these. The .NET package
carries its own native library, so that pairing is never yours to manage.

## 0.0.7 — a vocabulary is data

**Japanese header spellings are no longer read by default.** A column has
one canonical name and every other spelling is a vocabulary you supply.

```rust
// before
let channels = chdef::parse_ch_csv(text)?;
let table = chdef::ChTable::parse(text)?;

// after — for a file with Japanese headers
let japanese = chdef::ColumnVocabulary::japanese();
let channels = chdef::parse_ch_csv_with(text, &japanese)?;
let table = chdef::ChTable::parse_with(text, &japanese)?;
```

```csharp
// before
using var defs = Definitions.Parse(ch, bf);

// after
using var japanese = ColumnVocabulary.Japanese();
using var defs = Definitions.Parse(ch, bf, japanese);
```

A file whose headers are the canonical names needs no change. A file whose
headers are not recognised falls back to reading columns by position and
reports `header_assumed` — so the symptom of forgetting this is a
`header_assumed` in `issues`, not silence.

Also gone: `ColumnAliases` and `HeaderLanguage`, replaced by
`ColumnVocabulary`. `ChColumn::name(lang)` is `ChColumn::name()`, and
`with_columns(columns, language)` is `with_columns(columns, &vocabulary)`.

## 0.0.8 — two limits, and codes you can enumerate

`ChannelLayout::check_capacity()` answers with `Vec<Issue>` rather than
`Option<Issue>`: a layout can be over both its byte limit and its channel
limit, and learning one at a time means fixing one at a time. The ABI and
the .NET binding are unchanged — both already carried a list.

```rust
// before
if let Some(issue) = layout.check_capacity() { … }

// after
for issue in layout.check_capacity() { … }
```

In the .NET binding, `Channel.BitCount` is gone; the bits themselves are
`Channel.Bits` and the count is `Bits.Count`. `Reading` gained a `Bits`
member, so positional deconstruction of that record takes four elements.

The canonical CH header is 17 columns (`kind` appended). The 9- and
10-column positional forms read exactly as before — every column added
since is appended for that reason.

## 0.0.9 — a clamp stops being silent

No API changed. **A new Issue appears on traffic that has always encoded
out-of-range values**: `encode_value_clamped`, when a physical value the
channel width cannot hold reaches the wire as a different number.

The bytes are unchanged. What changes is `issues`, so a test asserting on
the count or on emptiness will notice. It is pointing at a place where the
number you asked for was silently not the number sent.

`IssueCode.All` (0.0.8) is how a table keyed by code stays complete as
codes are added.

## 0.0.10 — an observer is named for what it observes

```
check_capacity()            →  limits_exceeded()
chdef_layout_check_capacity →  chdef_layout_limits_exceeded
CheckCapacity()             →  LimitsExceeded()
```

One line per call site. Nothing else changed; since 0.0.8 the method had
answered for two limits and its name still said one.

## 0.0.11 — derived channels

Additive except for one record. `Channel` in the .NET binding gained
`Derived`, so positional deconstruction takes one more element; named
access is unaffected.

New, if you want it: a `derived` column, `kind = derived`, and
`seal` / `derived_mismatches` / `covered_bytes`. A definition set with no
derived channel behaves exactly as before — `encode` is untouched.

## 0.0.12 — documentation

No code changed. The readmes moved: `crates/chdef/README.md` is the
crates.io and docs.rs front page, `bindings/dotnet/Chdef/README.md` is the
NuGet one, and the repository readme points at both.

`BitReading` in the .NET binding gained `Name`, so a decoded bit carries
its name the way the Rust side always did. Positional deconstruction of
that record takes four elements.
