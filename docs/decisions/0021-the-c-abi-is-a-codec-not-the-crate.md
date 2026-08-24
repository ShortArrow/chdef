# ADR-0021: The C ABI is a codec, and it carries no enums

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.3

## Context

A consumer maintains a full C# reimplementation of this crate
(`SypfCore.ChdefCsv` / `ChdefCodec`) and pins it to the Rust one with the
golden vectors. Applying the vectors found three real divergences — the
C# reader identified columns by position instead of by header name, the
layout kept duplicate channel numbers, and a value using a channel's full
width was corrupted — plus a 32-bit ceiling on `default`. Every one is a
defect that cannot exist when there is one implementation, and the last
two are the same defect classes this crate found in itself (ADR-0014,
`256c0c3`).

The consumer's own framing of what would fix it: "parse → an opaque
layout handle → encode / decode over a buffer, even that much C ABI, and
this kind of bug structurally disappears."

Two questions had to be settled before writing any of it: how much of the
crate crosses the boundary, and who owns what on the other side.

## Decision

### It is a codec, not the crate

The first ABI exposes reading definitions, describing the layout, and
encoding / decoding frames — the part whose divergence produced wrong
numbers. The Table stage (`Grid`, editing, writing files back) is
deliberately **not** in it: it is the largest surface, it is the part the
C# side does not reimplement, and a frozen C surface is the most
expensive kind to have guessed wrong. It can be added when a consumer
edits definitions from C#.

### The ABI carries no enums

Every identity crosses as its **stable ASCII string** — an Issue's code,
a channel's `type`, a column's name — because that is what
`docs/spec/diagnostics.md` §2 already names as the stable identifier, and
because `IssueCode` and `DataType` are both `#[non_exhaustive]`. A C enum
of them would freeze a list that is documented as growing, and every new
code would need a table update in every binding before it could even be
displayed. Strings need none.

Numbers cross as numbers. The one exception is `endian`, an input with
exactly two values that cannot grow; it crosses as `0` / `1`.

### Every string chdef hands out goes into the caller's buffer

`chdef_*_text(..., char *buf, size_t cap) -> size_t` writes UTF-8 and
returns the length the value needs, truncating if it does not fit. No
string chdef produces is ever owned by the caller, so "who frees this"
is not a question the ABI has to answer, and the error path allocates
nothing.

### Absent numbers are negative

`row`, `col`, `channel` and `bit` are naturally non-negative, so they
cross as `int64` with `-1` for "not present" rather than doubling the
struct with presence flags. The header says so at each field.

### Handles are opaque, tagged, and freed once

A handle is a `Box` leaked into a pointer, carrying a tag word checked on
every entry, so a stale or wrong-typed pointer is reported as a status
rather than read. `chdef_*_free` takes the handle and is idempotent
against `NULL`.

### Every entry point catches panics

A Rust panic across `extern "C"` is undefined behaviour. Every function
wraps its body in `catch_unwind` and reports `CHDEF_PANIC`, rather than
the crate setting `panic = "abort"` and taking the host process down with
it — the host is the consumer's, not chdef's.

### The vectors run through the ABI

The golden vector harness runs a second time over the `extern "C"`
functions, so the ABI is verified to be the same implementation rather
than becoming a second one. That is the whole point of the exercise; an
ABI that could drift from the crate would reproduce the problem it
exists to remove.

### The header is checked in, and a test proves it complete

The C header is a hand-written artifact rather than a `cbindgen` build
step, and a test asserts every `extern "C"` symbol in the crate appears
in it. That keeps the header reviewable prose and still cannot silently
fall behind.

### The .NET binding is not chdef's

chdef ships the cdylib and the header. The P/Invoke declarations, the
NuGet packaging and the per-platform binaries need a .NET toolchain in CI
and a .NET reviewer; declaring them here would put a language this
repository does not build into its release path. What matters is that the
*logic* stops being duplicated — declarations are not logic.

## Alternatives rejected

- **A .NET package as the deliverable**: either it bundles a native
  binary, which is a C ABI plus packaging, or it is managed code, which
  is the duplication being removed.
- **Exposing the whole crate**: `ChannelDef` alone has fourteen fields,
  and the editing surface is larger than the codec. A frozen C surface
  guessed wrong is the most expensive kind.
- **A thread-local last error**, errno style: hidden process-wide state
  in a library that has none anywhere else.
- **chdef allocating the strings it returns**, with a `chdef_free_string`:
  one more thing every binding must get right, in exchange for saving the
  caller a length query.
- **Numeric enums for codes and types**, for cheap filtering: freezes a
  list both specifications call open, and the identity the specification
  gives is the string.

## Consequences

- A new workspace member holds the ABI, so the crate itself stays free of
  `extern "C"` and `unsafe`.
- The three divergences the vectors found in the C# copy stop being
  possible for anything that goes through the ABI.
- The ABI's own surface is now frozen in the same way the crate's is: the
  functions in the header are a promise, and the ADR-0005 reasoning
  applies to them from their first release.
