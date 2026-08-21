# Architecture Decision Records

One decision per file. An ADR names the files and sections it affects; the
specification and the README never point back at an ADR.

| ADR | Date | Title | Status |
|---|---|---|---|
| [0001](./0001-extract-ch-concept-from-chbridge.md) | 2026-08-18 | Extract the CH / BF concept of chbridge into a standalone crate | Accepted |
| [0002](./0002-resolve-divergent-rules.md) | 2026-08-18 | Settle the CH / BF rules on which consumers had diverged | Accepted |
| [0003](./0003-language-neutral-column-names.md) | 2026-08-18 | Column names are language-neutral; English and Japanese spellings are both canonical | Accepted |
| [0004](./0004-input-boundary-is-text.md) | 2026-08-19 | The crate's input is text; the path is a convenience, and the file dialog is the consumer's | Accepted |
| [0005](./0005-freeze-line-before-first-publish.md) | 2026-08-20 | The public surface is the crate root, and data types tolerate the growth the specification already promises | Accepted |
| [0006](./0006-bounds-are-carried-not-applied.md) | 2026-08-21 | min / max are physical bounds with a raw escape, carried but never applied silently | Accepted |
| [0007](./0007-raw-bytes-primitive-truncates.md) | 2026-08-21 | The raw→bytes primitive truncates; clamping belongs to the physical storey | Accepted |
| [0008](./0008-layout-checks-have-no-rows.md) | 2026-08-21 | build_layout returns Issues, and its Issues carry no row | Accepted |
