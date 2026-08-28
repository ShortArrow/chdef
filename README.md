# chdef

🌐 **English** | [日本語](docs/README.jp.md)

[![CI](https://github.com/ShortArrow/chdef/actions/workflows/ci.yml/badge.svg)](https://github.com/ShortArrow/chdef/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Channel definitions (CH) and bit-field definitions (BF) for binary
> frames: parse the CSVs, compute the layout, convert raw ↔ physical.

> **⚠ Pre-alpha (0.0.x).** The API and the CSV rules are still moving;
> until 0.1.0 lands, patch releases may break either.

---

A **CH definition** (channel definition CSV) gives every field of a binary
frame a meaning; a **BF definition** (bit-field definition CSV) gives every
bit of a `BF`-typed channel a name. This repository holds one
implementation of both and every path to it, so that every consumer reads
the same file the same way.

## What is here

| Package | Version | Calling from | What it is | Install |
|---|---|---|---|---|
| [`chdef`](./crates/chdef/README.md) | [![crates.io](https://img.shields.io/crates.io/v/chdef)](https://crates.io/crates/chdef) | Rust | the library | `cargo add chdef` |
| [`chdef-capi`](./crates/chdef-capi/README.md) | [![crates.io](https://img.shields.io/crates.io/v/chdef-capi)](https://crates.io/crates/chdef-capi) | C, C++ | a C ABI over it | `cargo add chdef-capi` |
| [`Chdef`](./bindings/dotnet/Chdef/README.md) | [![nuget](https://img.shields.io/nuget/v/Chdef)](https://www.nuget.org/packages/Chdef) | C#, .NET | the .NET binding, native libraries included | `dotnet add package Chdef` |
| [`@shortarrow/chdef`](./bindings/js/README.md) | [![npm](https://img.shields.io/npm/v/@shortarrow/chdef)](https://www.npmjs.com/package/@shortarrow/chdef) | JavaScript, TypeScript | the JavaScript binding, WebAssembly and TypeScript declarations included | `npm install @shortarrow/chdef` |
| [`chdef-core`](./crates/chdef-core/README.md) | [![crates.io](https://img.shields.io/crates.io/v/chdef-core)](https://crates.io/crates/chdef-core) | firmware in Rust or C | the raw-only rules for a device, `no_std`, with C entry points | `cargo add chdef-core` |
| [`chdef-gen`](./crates/chdef-gen/README.md) | [![crates.io](https://img.shields.io/crates.io/v/chdef-gen)](https://crates.io/crates/chdef-gen) | a firmware build, either language | expands a definition into a constant table for Rust or C | `cargo install chdef-gen` |

Each has its own readme, aimed at the language you are calling from. The
C ABI carries every rule the specification states, so a consumer in C or
C# never writes one of them a second time; the JavaScript binding reaches
the same rules through WebAssembly.

## Documentation

| | |
|---|---|
| [docs/guide.md](./docs/guide.md) | The shortest path through each task |
| [docs/spec/](./docs/spec/README.md) | What the format is, exactly |
| [docs/migration.md](./docs/migration.md) | What changed between 0.0.x releases |
| [docs/decisions/](./docs/decisions/README.md) | Why it is the way it is |

Every example in the Rust readme is compiled and run by `cargo test`;
every example in the .NET readme is a test in `Chdef.Tests`, checked
against the readme by the workspace. A page that cannot rot is the point.

## Origin

The CH / BF concept was extracted from `chbridge-core` of chbridge, an
internal telemetry bridge. Definition files themselves (real-device
channel tables) belong to each consumer; this repository holds synthetic
data only.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
