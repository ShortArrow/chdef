# ADR-0034: The target holds the layout, not the file

- Status: Accepted
- Date: 2026-08-28
- Release: 0.0.16

## Context

chdef reads a definition file at run time: the CSV is parsed, columns
are identified through a vocabulary, findings are reported, and a
layout is built. Every consumer so far — the crate, the C ABI, .NET,
JavaScript — runs on a host with an allocator and a file.

A microcontroller sending or receiving these frames has neither. It
also has no use for most of what the file carries: names, units,
physical scaling, diagnostics. What it needs is the layout — where each
channel is, how wide, what its default is, which bytes its CRC covers —
and that is fixed when the firmware is built.

Three shapes were considered.

Running `chdef` on the target with `alloc`: possible, since the crate
uses only `Vec` and `String`, but it drags the CSV parser, the
diagnostics and floating-point scaling onto a device that will never
call them, and it parses at boot a file that could not have changed.

Generating C code — a table and a small codec — from the CSV: the
firmware build stays C-only, but the codec is a second implementation
of `docs/spec/conversion.md` §3 and `format.md` §6, the thing ADR-0023
exists to prevent. The golden vectors could be run through it on a host
compiler, which catches divergence but does not remove it.

Splitting the raw-only rules into a `no_std` core that both the host
crate and the target use: one implementation, certified by the vectors
through every host path. The cost is that a C firmware build links a
Rust static library, which needs a Rust toolchain for the target.

## Decision

**`chdef-core` holds the rules a device needs, and `chdef` calls it.**
Positions, raw ↔ bytes at every width in both byte orders, the merged
default of conversion.md §4, and the CRC of format.md §6 live in
`crates/chdef-core`, `#![no_std]`, with no dependency, no allocator and
no floating point. `chdef` depends on it and delegates the same
arithmetic, so there is one implementation.

**A definition is expanded at build time, by `chdef-gen`.** The
generator parses the file as the host does and writes the layout as a
constant table — Rust source or a C header. A definition that produces
any Issue is refused; the target has nowhere to report one.

**The target is raw-only.** Physical values, `lsb` and `offset` stay on
the host. A device that needs to scale does so in its own code, over
the raw values the core hands it; the question of `f32` against the
host's `f64` never arises in chdef.

**C reaches the core through `chdef_core_*` entry points** on the same
crate, built as a static library for the firmware's target, and not
through generated C.

## Consequences

A device and a host agree on a frame because they ran the same code.
The vectors, which already certify the host paths, certify the core
through them.

A C firmware build acquires a Rust toolchain for its target. A project
that cannot take one has the generated table and the specification, and
writes its own codec — the shape rejected above, available as a fallback
because the table is data.

`chdef`'s public surface does not change; `Crc::of`,
`raw_from_bytes_endian` and `raw_to_bytes_endian` keep their signatures
and call the core underneath.

The release grows a fourth artifact: `chdef-core` and `chdef-gen` on
crates.io beside `chdef` and `chdef-capi`.
