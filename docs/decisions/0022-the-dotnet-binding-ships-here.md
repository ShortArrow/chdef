# ADR-0022: The .NET binding ships from here, and its vectors run in this CI

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.3
- Supersedes the "The .NET binding is not chdef's" decision of
  [ADR-0021](./0021-the-c-abi-is-a-codec-not-the-crate.md); every other
  decision in that record stands.

## Context

ADR-0021 shipped a C ABI and a header, and left the P/Invoke
declarations, the packaging and the per-platform binaries to the
consumer, on the reasoning that "what matters is that the *logic* stops
being duplicated — declarations are not logic".

That reasoning fails on two counts.

**An ABI nobody can call is a facility, not a deliverable.** The C#
reimplementation exists because calling Rust from C# was work nobody had
done; shipping a `.h` and a `.so` leaves that work exactly where it was.
The duplication is removed the day the C# project calls the ABI, not the
day the ABI exists.

**And a hand-written binding is a worse divergence than the one being
removed.** The divergences the golden vectors found in the C# copy
produced wrong numbers. A mis-declared struct field or a wrong integer
width in P/Invoke produces silent memory corruption, and no vector
catches it because the process is already broken. Handing the
declarations to the consumer trades a detectable class of defect for an
undetectable one.

What ADR-0021 was right about is narrower: NuGet packaging with native
binaries per platform is a distribution concern with real cost, and it is
separable from the declarations. It is not, however, optional — a binding
a consumer must vendor by hand is most of the same problem.

## Decision

- **A .NET binding ships from this repository**, targeting `net8.0`:
  the P/Invoke declarations against `chdef.h`, and a small safe wrapper
  that owns the handles (`IDisposable`), does the two-call buffer dance
  for every string, and turns a status into an exception.
- **A structural test proves the declarations match the ABI** without a
  .NET toolchain: every `extern "C"` function, every `CHDEF_*` constant
  and every `repr(C)` field must appear in the C# source, in order and
  with the mapped type. The same discipline the header test uses.
- **The golden vectors run through the C# binding in this CI.** A
  `dotnet` job loads the built native library and runs
  `docs/spec/interchange.md` §3 over the managed API, so the vectors
  certify the path the C# project actually takes rather than a path
  beside it. This is the decision that makes the whole exercise land.
- **A NuGet package carries the native binaries** under
  `runtimes/<rid>/native/`, so a consumer writes `dotnet add package` and
  nothing else. Each one is built on a runner of its own OS and
  architecture (`linux-x64`, `win-x64`, `osx-arm64`, `osx-x64`):
  cross-building a cdylib needs the target platform's linker, and a native
  runner is both simpler and closer to what ships.
- **It is published the way the crate is: OIDC trusted publishing**, no
  long-lived secret in this repository. The .NET side follows the layout
  ivi-cli settled on — `global.json` pinning the SDK, a repository-level
  `Directory.Build.props` and central package management, committed lock
  files restored with `--locked-mode`, `ContinuousIntegrationBuild` under
  CI so the package is reproducible, and SourceLink so its symbols point
  back here.

## Alternatives rejected

- **Shipping only the header** (ADR-0021's decision): the reasoning
  above.
- **Shipping the `.cs` without packaging**: a file every consumer
  vendors, updates by hand, and diverges from at their own pace.
- **A source generator over `chdef.h`**: a build-time C parser to avoid
  writing two hundred lines once, and a second thing to review.
- **Testing the binding only in the consumer's repository**: the
  vectors' whole value is that they live with the definition of correct.

## Consequences

- This repository's CI gains a `dotnet` toolchain and its release path
  gains a NuGet feed. That is the cost ADR-0021 declined to pay and the
  reason it was wrong to decline: the cost is real and the value is the
  entire point of the ABI.
- The ABI's surface is now frozen from two directions — the header and
  the C# declarations — so adding to it means touching both, and the
  tests say so.
- `SypfCore.ChdefCsv` / `ChdefCodec` can be deleted rather than pinned.
