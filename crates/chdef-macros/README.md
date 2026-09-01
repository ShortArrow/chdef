# chdef-macros

A CH / BF definition expanded into the constant table where you write it,
with no generated file to commit or regenerate:

    chdef_macros::layout!("ch.csv");
    chdef_macros::layout!("ch.csv", bf = "bf.csv", endian = big, japanese);

The spelling is `chdef_macros::layout!` and not `chdef::layout!` because
the macro parses the file through `chdef`, which therefore cannot depend
on the macro (ADR-0035).

The options come in any order, each at most once: `bf` names the bit-field
file, `endian` is `little` or `big` and defaults to `little`, `japanese`
reads the column spellings of `docs/spec/format.md` §2. Both paths are
relative to the invoking crate's `CARGO_MANIFEST_DIR`, the one directory a
procedural macro can know on stable Rust; the expansion embeds the files
with `include_bytes!`, so an edit to a definition rebuilds the crate and a
stale table cannot outlive it.

## What it declares

The items `chdef-gen --rust` writes: `LAYOUT`, a `chdef_core::Layout` the
core's own calls take, and one `CH_…` constant per named channel. The file
and the expansion are the same text; the macro adds nothing. Because those
items name `chdef_core::…`, the invoking crate depends on `chdef-core`
too, beside this crate: this one runs on the host at compile time, the
core on the target.

## A refused definition is a compile error

Every finding `chdef-gen` would have printed — the row, the column, the
code — comes back as the message of a `compile_error!`. A row the host
would load with a warning does not reach a device, where nothing can warn.
