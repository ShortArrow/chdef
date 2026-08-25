# ADR-0030: The JavaScript binding ships from here, and binds the crate rather than the C ABI

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.15

## Context

chdef already ships a C ABI and a .NET binding over it. A consumer now
wants the same rules in a browser: a single-page application that edits
definition files and sends the frames they describe.

The obvious economy is to reuse what exists. WebAssembly can call a C
ABI, and the C ABI is the layer that was built precisely so a second
language would not reimplement the rules.

That economy does not survive contact with the target. The C ABI's shape
is dictated by what C can carry: a string leaves through a two-call
buffer dance, a handle is an opaque pointer with a tag, and no
enumeration crosses at all. Every one of those constraints exists because
C has no better option. `wasm-bindgen` does: it carries strings, vectors
and structs itself, and it emits the TypeScript declarations from the
Rust signatures. Going through the C ABI would mean reimplementing, in
JavaScript, the buffer dance the .NET binding already had to write —
paying C's costs in a language that is not C.

Against binding the crate directly: it is a fourth path through the same
rules, and a fourth path is a fourth place to be wrong.

## Decision

- **A JavaScript binding ships from this repository**, as the crate
  `chdef-wasm` built with `wasm-bindgen`, published to npm as `chdef`.
- **It binds the crate, not the C ABI.** The C ABI stays what it is: the
  path for C and for .NET.
- **It defines its own record types.** `chdef::Decoded` borrows the
  layout and the frame it was read from, and a borrowed type cannot leave
  the module, so a reading is copied into a plain value on the way out —
  the same shape the .NET binding takes, and the same reason.
- **The package ships two builds from the one crate**: `bundler` for Vite
  and the other bundlers, `nodejs` for Node. They come from the same
  Rust source, so they cannot disagree.
- **The golden vectors run through it**, as they run through the crate,
  the C ABI and the .NET binding. This is what makes the fourth path
  affordable: it is measured against the same files, so it cannot drift
  into agreeing with itself.

## Consequences

The rules exist once in Rust and are reachable from C, C#, and now
JavaScript and TypeScript. A browser application gets the frame layout,
the conversions and the diagnostics without a fourth reimplementation.

The cost is a second binding to keep building and releasing, and a
WebAssembly toolchain (`wasm-pack`, the `wasm32-unknown-unknown` target)
in CI. The vectors bound the risk; the build cost is real and recurring.

A consumer who wants chdef from C or C# is unaffected — the C ABI has
not moved.
