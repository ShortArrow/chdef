# ADR-0002: Settle the CH / BF rules on which consumers had diverged

- **Status**: Accepted
- **Date**: 2026-08-18
- **Release**: 0.0.x (specified in 0.0.1; implemented incrementally)

## Context

Several implementations reading the same CH / BF CSV disagreed on the points
below. chdef has to pick one answer for each and write it into the
specification.

## Decision

| Point | Decision | Reason |
|---|---|---|
| Decimal `値(デフォルト)` | **Raw value** (both `0x` and decimal notation are raw) | In practice the default column holds bit patterns such as SYNC or fixed markers. If a physical-value default is ever needed, a separate column is safer |
| Empty BF `値(デフォルト)` | **Unspecified = keep the parent channel default's bit**. Anything other than `0` / `1` is also unspecified + Issue | "Empty keeps the underlying bit" is easy to explain; defaulting to `0` silently clears bits |
| Negative LSB | **Allowed** (0 / empty → 1; only NaN / infinite → 1 with an Issue) | An inverted physical quantity can be meaningful. No reason to reject it |
| Duplicate `番号` / `(番号, BIT番号)` | Layout keeps the **first row + Issue**; Rows keeps them all | Consumers that want the set of rows (sequence checks etc.) get Rows |
| Decoding a short frame | Overrunning channels are **omitted from the result** (not zero-filled) | Never show a value that did not arrive as if it did |
| Physical → raw rounding | **Half away from zero** | Simple and easy to match across languages |
| Upper bound of `BIT番号` | **Below the parent width (`バイト数 × 8`)**. Beyond it the row is dropped with an Issue | The bound follows from the parent width; it is not a constant |
| Identifying columns | **Header name + aliases**. Without a header, the first 9 columns are taken in canonical order with an Issue | One reader accepts forms with different column counts and English headers |
| `型` and width | **`型` is interpretation only (UI / SI / BF); the width is always `バイト数` (1–8)**. A disagreeing suffix is an Issue | 64-bit fits naturally and there is a single source of width |
| Filling gaps, enforcing consecutive numbers, merging several CSVs, checking packet length | **Out of scope** (only Rows and the `capacity` check are provided) | These are policies that depend on the packet structure, not mechanisms |
| `表示形式`, `値(最小/最大)`, `お気に入り`, `変数名`, `備考` | Read, **kept and emitted in JSON**. The only interpretation is DEC / HEX for `表示形式` and the `HEX + LSB≠1` Issue | The minimum so that consumers with a GUI are not stuck |
| Diagnostics | Report **Issues (code + row / column + English message)** and never stop loading. Only I/O and CSV structure errors stop it | Consumers can implement "swap in only if it loaded completely" |

## Consequences

- The above is written as rules in `docs/spec/format.md` / `layout.md` /
  `conversion.md` / `diagnostics.md`.
- Changing any of these decisions means superseding this ADR with a new one.
