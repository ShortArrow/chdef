# ADR-0004: The crate's input is text; the path is a convenience, and the file dialog is the consumer's

- **Status**: Accepted
- **Date**: 2026-08-19
- **Release**: 0.0.2

## Context

Choosing a CH / BF file is a GUI act. The dialog differs per platform, it
carries permissions on Android and iOS, and on a browser it does not exist at
all. chdef has no business knowing any of that, and it does not: nothing in
the crate refers to a GUI. The friction is one layer lower, in what the two
sides hand each other.

`load_ch_csv(path: &str)` and `load_bf_csv(path: &str)` say that the input
arrives as a filesystem path spelled in UTF-8. Three consumers already
contradict that:

- Avalonia's `IStorageFile`, Android's Storage Access Framework and a
  browser `<input type="file">` return a stream or a byte array. A path
  either does not exist or is not the thing the user picked.
- A Windows path is a sequence of UTF-16 code units and need not be valid
  Unicode. `&str` cannot hold one that is not; `Path` can.
- A WASM consumer has no `std::fs`, so `load_*` is dead weight there.

Encoding is the same boundary seen from the other side. `docs/spec/format.md`
fixes the file encoding at UTF-8, while CH CSVs exported by spreadsheet
software in Japan are CP932. Someone has to decode, and the question is who.

## Decision

- `parse_ch_csv` / `parse_bf_csv` are the crate's entry points. They take
  text the caller has already decoded, and they are the only functions the
  specification describes.
- Add `parse_ch_csv_bytes(&[u8])` / `parse_bf_csv_bytes(&[u8])` for the
  consumer that holds a byte array and no path. A BOM is a byte-level fact,
  so these strip it; the text entry points keep stripping it for callers who
  decoded without removing it.
- `load_ch_csv` / `load_bf_csv` take `impl AsRef<Path>` and remain thin
  wrappers over `std::fs`. They are the only place in the crate that touches
  the filesystem, which leaves them behind a `std` feature if a WASM
  consumer ever needs one.
- Decoding a non-UTF-8 file is the consumer's work. chdef does not depend on
  `encoding_rs` and does not guess an encoding.
- The dialog, its permission handling and its platform result type stay in
  the GUI, which reads them down to bytes before calling chdef.

## Alternatives considered

- **Take `impl Read` instead of bytes**: the parser reads every record before
  it can tell a header row from a data row, so streaming buys nothing, and
  `Read` drags `std` into the signature that WASM needs to keep clean.
  Rejected.
- **Decode inside chdef with `encoding_rs`**: the crate would guess an
  encoding on behalf of consumers whose files differ by country, and the
  guess would override a rule the specification already states. Rejected.
- **Keep `&str` paths and let the GUI write a temporary file**: works, and
  makes every Android and browser consumer pay for a round trip to disk to
  reach a parser that only wanted the bytes. Rejected.

## Consequences

- `crates/chdef/src/csv.rs` gains the two byte entry points and moves BOM
  stripping to them. `read_to_string` becomes the crate's only `std::fs`
  call.
- `docs/spec/format.md` §1 states that chdef reads UTF-8 and that decoding
  from anything else happens before the call.
- A consumer holding a stream reads it to a `Vec<u8>` itself. chdef will not
  grow a reader-based API, so a later request for one is a change to this
  decision rather than an addition.
