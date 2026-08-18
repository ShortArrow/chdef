# ADR-0003: Column names are language-neutral; English and Japanese spellings are both canonical

- **Status**: Accepted
- **Date**: 2026-08-18
- **Release**: 0.0.2

## Context

The CH / BF CSV format originated with Japanese header names
(`番号,バイト数,…`), and every existing file is written that way. chdef is a
crate for a global audience: requiring a Japanese header to use it would be
unacceptable, but breaking every existing file would be too. Cell values are
already ASCII (`UI` / `SI` / `BF`, `DEC` / `HEX`, `0x…`, `1` / `true`), so the
header is the only place where the language shows.

## Decision

- A column is an identifier, not a string: `ChColumn` (16 variants) and
  `BfColumn` (5 variants). Everything inside chdef refers to columns by
  identifier.
- Every column has **two canonical spellings** — English (lower-case ASCII:
  `number, bytes, bits, section, name, type, lsb, offset, unit, min, max,
  default, memo, var, format, favorite` / `number, bit, name, default, memo`)
  and Japanese (the original form) — plus aliases. The reader trims and
  matches header cells case-insensitively and accepts both languages, even
  mixed within one header.
- The writer keeps the header it read. For a new file the header language is
  a parameter (`HeaderLanguage`); the default is English.
- Documentation names columns by their English spelling and gives the
  Japanese spelling in parentheses on first use.

## Alternatives considered

- **Japanese only** (status quo): unusable outside Japan. Rejected.
- **English only, with a migration of existing files**: every existing file
  and every tool that writes them would have to change at once. Rejected.
- **Content-based detection of the header language**: unnecessary once
  spellings map to identifiers; a spelling table is simpler and testable.
  Rejected.

## Consequences

- `docs/spec/format.md` §2–§4 list both spellings; `crates/chdef/src/columns.rs`
  is the single source of the spelling table.
- Adding a language later means adding a spelling column, not a new reader.
