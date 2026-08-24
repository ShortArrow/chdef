# ADR-0017: The shape of a file is read from it, not imposed on it

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.3

## Context

ADR-0009 set the round-trip guarantee at cell granularity and normalised
everything else to the write rules of `docs/spec/format.md` §1: a
byte-order mark and `\r\n`, whatever the source had. The reasoning was
that remembering the source's shape buys only "a quieter diff for files
no spreadsheet has touched — Excel normalises all of it anyway on first
save".

A consumer integrating chdef reported the cost within days. Definition
CSVs live in their repository with `\n` endings and no BOM. Editing one
cell rewrote every line of the file and added a mark to the first, so the
diff of a one-cell change was the whole file. That is not a cosmetic
complaint: a review of a definition change becomes unreadable, and the
"files no spreadsheet has touched" case turned out to be the common one,
not the rare one.

The reasoning was also narrower than the decision. Remembering *per-cell
quoting* is the expensive part; remembering whether the file has a BOM
and which separator it uses is two values for the whole file.

## Decision

- **`CsvStyle { bom: bool, line_ending: LineEnding }`** is read from the
  file a table parses and used when it writes. A table created in code
  uses `CsvStyle::default()` — the write rules of format.md §1 — so a new
  file still gets a BOM and `\r\n`.
- **The separator is detected quote-aware**: a newline inside a quoted
  cell is part of the cell (format.md §1) and says nothing about how the
  file separates records. The scan that already looks for an unterminated
  quote reports both, in one pass.
- **`style()` / `set_style()`** expose it, so a project that wants one
  shape across every file can impose it deliberately rather than having
  chdef impose it silently.
- **Quoting stays normalised.** A cell is quoted when the write rules say
  it must be and not otherwise; the source's unnecessary quotes are still
  dropped. Remembering per-cell quoting would need a parallel structure
  that every edit has to keep honest, for a diff line here and there.

The guarantee of ADR-0009 therefore widens: rows and cell contents, plus
the file's shape. A file that already follows the write rules now
round-trips byte for byte.

## Alternatives rejected

- **An LF-only alternative to `to_csv`**: two writers to keep in step,
  and it answers only one of the two differences.
- **Remembering the source verbatim and splicing edits into it**: the
  strongest possible guarantee, and it makes every structural edit —
  inserting a row, renumbering — a text-manipulation problem instead of a
  grid one.
- **Leaving it and telling consumers to normalise their repositories**:
  moving chdef's default onto every consumer's version control.

## Consequences

- `docs/spec/editing.md` §2 states the wider guarantee, and the byte-for-
  byte case is a test.
- Breaking for anything that assumed `to_csv` always emits a BOM: it now
  emits one only if the source had one, or the caller asks.
- The `csv` scan pays for one extra `bool` and one `Option`, on a pass it
  was already making.
