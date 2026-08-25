# ADR-0031: A record crosses to JavaScript as an object, and only state is a handle

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.15

## Context

`wasm-bindgen` has one obvious way to carry a struct: annotate it and it
becomes a JavaScript class. The class is a handle into linear memory, and
the fields are getters that reach across.

Applied to every type, that produces a binding whose values behave
unlike JavaScript values:

- Each one holds memory until `free()` is called, so decoding a frame
  leaks one object per channel per frame unless the caller frees each.
- `JSON.stringify` yields `{"__wbg_ptr":1116096}`, and spreading yields
  the same. The pointer is the whole object.
- `structuredClone` refuses it. A page that decodes frames in a worker
  cannot post the readings back, which is the shape a browser
  application reaches for as soon as the frames arrive faster than the
  main thread wants to work.

The distinction the annotation flattens is between a **handle** and a
**record**. `Definitions`, `Table` and `ColumnVocabulary` own Rust state:
they have identity, they are mutated, and freeing them means something. A
channel, a reading, an Issue is a snapshot of what was read — it owns
nothing, and its identity is its contents.

Against carrying records as plain objects: the conversion goes through
serde, which is a dependency and a serialization step per call, and the
TypeScript declarations no longer come from `wasm-bindgen` alone.

## Decision

- **A handle is a class; a record is a plain object.** `Definitions`,
  `Table` and `ColumnVocabulary` are the only three classes, because they
  are the only three things that own state.
- **The TypeScript declaration for a record is generated from the Rust
  struct** by `tsify`, so the binding holds no second, hand-written
  declaration to drift from the first. The generated `.d.ts` is
  unchanged by this decision — a record was already declared with its
  fields; it is now an `interface` rather than a `class`.
- **The conversion is carried by `Ts<T>` rather than the
  `into_wasm_abi` / `from_wasm_abi` attributes**, which leak the
  allocation when serialization fails. `Ts<T>` forwards the value and
  converts inside the function, where a failure is an ordinary `Err` and
  destructors run.
- **A value arriving from JavaScript that does not have the declared
  shape is thrown back.** JavaScript is dynamic and a caller may pass
  anything; reading a missing field as zero would put a number on the
  wire that nobody asked for.

## Consequences

A caller may keep a record, spread it, diff it, store it, or post it to a
worker, and never has to free one. A `Definitions` still must be freed,
which is now a claim about three named classes rather than about
everything the binding returns.

`serde`, `serde-wasm-bindgen` and `tsify` are dependencies of the
WebAssembly crate. They are compile-time and conversion-layer only: they
appear in no signature the binding exposes, so replacing them would not
move the JavaScript API.
