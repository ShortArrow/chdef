# ADR-0035: The layout macro is its own crate, beside the core

- Status: Accepted
- Date: 2026-09-01
- Release: 0.0.17

## Context

ADR-0034 expands a definition at build time with `chdef-gen`, a command
that writes a file. A Rust firmware crate then carries a generated file
it did not write and has to regenerate when the CSV changes. A
procedural macro removes the file: `layout!("ch.csv")` expands to the
same constants in place, and the crate rebuilds when the CSV does.

Where the macro lives is constrained by the dependency graph. It has to
parse the CSV, which is `chdef`'s job, and `chdef-gen` already turns a
parsed definition into Rust source; so the macro depends on `chdef-gen`,
which depends on `chdef`, which depends on `chdef-core`. Cargo refuses a
cycle, and a procedural-macro crate is a dependency like any other: a
macro re-exported from `chdef` or from `chdef-core` would make one of
them depend on the crate that depends on them. `chdef::layout!` and
`chdef_core::layout!` are therefore not available spellings.

## Decision

**The macro is the crate `chdef-macros`, invoked as
`chdef_macros::layout!`.** A firmware crate depends on it beside
`chdef-core`; the macro runs on the host at compile time, the core on
the target.

**It expands what `chdef-gen` writes.** The items are those of
`chdef_gen::rust_source`, wrapped in a module and re-exported, so a
`--rust` file and a `layout!` expansion are the same text and the same
constants. The macro adds nothing the file does not have.

**Paths are relative to the invoking crate's `CARGO_MANIFEST_DIR`.**
That is the one location a procedural macro can know on stable Rust,
and the convention every other file-reading macro follows.

**A refused definition is a compile error** carrying the same findings
`chdef-gen` prints — the row, the column, the code.

**The CSV is a build input.** The expansion embeds the file's bytes
with `include_bytes!`, so Cargo re-runs the macro when the file changes;
a stale table cannot survive an edit to the definition.

## Consequences

A Rust firmware crate has no generated file to commit or regenerate.
A C firmware build is unchanged: it has no macro to run and keeps the
header `chdef-gen --c` writes.

The release grows a fifth crate. `chdef-macros` is published after
`chdef-gen`, on which it depends by version.

The two spellings a reader might reach for first, `chdef::layout!` and
`chdef_core::layout!`, do not exist, for the reason above; the crate
front pages say which spelling does.
