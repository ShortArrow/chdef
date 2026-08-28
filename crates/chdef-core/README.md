# chdef-core

The rules a device needs from chdef: where each channel sits in a frame,
its raw value out of one and into one in either byte order, every
channel's default, and the CRC a derived channel computes over the bytes
it covers. `#![no_std]`, no dependency, no allocator, no floating point
and no panic on any input.

## Who this is for

A microcontroller. It does not read a CH CSV — it holds the layout the CSV
describes, fixed when the firmware was built — so parsing, vocabularies,
diagnostics, physical values and the grid are not here. A host wants
`chdef` instead, which reads the files and delegates this same arithmetic
here, so a device and a host reading the same frame agree because they ran
the same code (ADR-0034).

## The table

`chdef-gen` reads a definition and writes the layout out as a constant:
Rust source declaring a `chdef_core::Layout`, or a C header declaring the
tables and a `CHDEF_LAYOUT`. A definition with any Issue is refused, since
a device has nowhere to report one. Nothing here parses anything.

## Building

For Rust firmware, as a dependency built for the firmware's target:

    cargo build -p chdef-core --target thumbv7em-none-eabihf

For C firmware, as a static library carrying the `chdef_core_*` entry
points of `include/chdef_core.h`:

    cargo rustc -p chdef-core --features c --target thumbv7em-none-eabihf \
        --release --crate-type staticlib -- -C panic=abort

The archive lands under `thumbv7em-none-eabihf/release/libchdef_core.a`.
The `c` feature carries a `#[panic_handler]` for bare targets only, so
Rust firmware with its own is unaffected. `docs/spec/embedded.md` states
what runs on the target and what does not.
