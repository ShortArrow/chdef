# Changelog

All notable changes to **chdef** are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The 0.0.x line treats each `0.0.x → 0.0.(x+1)` bump as MAJOR-equivalent
(Cargo's pre-1.0 convention): breaking changes are allowed within 0.0.x and
announced under `### Breaking`. The trunk is `main`; a release is a `vX.Y.Z`
tag, published to crates.io.

## [0.0.12] - 2026-08-25

### Fixed
- **The front page was wrong, in both registries.** Its Rust example did
  not compile — `parse_ch_csv` answers with `Parsed`, and the example fed
  that straight to `build_layout`, which takes a `Vec` — and the crate's
  own first line still said columns are "spelled in English or Japanese",
  which stopped being true in 0.0.7. Both had rotted because nothing
  compiled them.

### Added
- **Every example on a readme is executed.** The Rust one is the crate's
  doc page (`include_str!`), so `cargo test` compiles and runs each block;
  the C# one is mirrored by `Chdef.Tests/ReadmeTests.cs` with a checker in
  the workspace that fails the build if the page and the tests disagree.
  C# has no doctest, so the mirror is what stands in for one.
- [docs/guide.md](docs/guide.md): the shortest path through reading,
  sending, receiving, asking and editing — and what chdef will not do for
  you, each with the reason.
- [docs/migration.md](docs/migration.md): every break from 0.0.5 onward in
  one place, for someone jumping several releases at once.
- A readme per audience. `crates/chdef/README.md` is the crates.io and
  docs.rs front page, `bindings/dotnet/Chdef/README.md` the NuGet one, and
  the repository readme points at both. The NuGet page had been showing
  Rust code.
- The specification prints the CRC catalogue and the Issue codes, and a
  test compares both against what the crate ships. It found a code
  documented nowhere on its first run.
- Japanese counterparts of the guide and the migration notes.

### Breaking
- `BitReading` in the .NET binding gained `Name`. The Rust side has always
  handed the whole definition over beside the value; the binding handed
  over a number and left the caller to look the name up, which only became
  visible when the readme example was written against it. Positional
  deconstruction of that record takes four elements.

## [0.0.11] - 2026-08-25

### Added
- **Derived channels.** `kind` gains `derived`, reserved since 0.0.8 and
  now meaning something: a channel chdef computes from the rest of the
  frame by a recipe the file states in a new `derived` column, as
  `crc16/x25 1..7`.
- **The coverage is stated, never assumed.** A frame of sync, length, data
  and CRC may cover the data alone, the length and the data, or everything
  before the CRC; which one a device means is a property of its protocol,
  not of CRC. A recipe without a range is `derived_invalid`. Spans may be
  listed — `2..3,5..7` — and they name channels rather than bytes, so
  inserting a channel moves the coverage with it.
- **A recipe is its six numbers; a name is a shorthand.** Width,
  polynomial, initial value, input reflection, output reflection and final
  XOR describe every CRC. `crc16/x25` expands to exactly those, and a
  device whose CRC is in no catalogue writes the numbers instead. Six
  variants ship, each checked against its published check value.
- **`ChannelLayout::seal`** fills every derived channel; `encode` never
  does, so ADR-0025's "kind is a mark, not a behaviour" stands and a
  consumer with no derived channel sees no change.
  **`derived_mismatches`** is the check a receiver makes.
- **`covered_bytes`** is the storey below sealing: a device whose checksum
  chdef does not compute still says which bytes it covers, so a caller
  runs its own over exactly those. The algorithm is replaceable without a
  trait, a registry, or a callback across the ABI.
- All of it across the ABI and the .NET binding — `chdef_seal`,
  `chdef_derived_mismatches`, `chdef_covered_bytes`,
  `chdef_recipe_count` / `_name`, and `Definitions.Seal` /
  `DerivedMismatches` / `CoveredBytes` / `Recipes`.

### Breaking
- `Channel` in the .NET binding gains `Derived`, so positional
  deconstruction takes one more element.
- `CHDEF_ABI_VERSION` is `6`.

## [0.0.10] - 2026-08-25

### Added
- **Asking whether a value is inside its channel's declared range**, in
  the three places a value can be: `ChannelLayout::values_out_of_range`
  before sending, `readings_out_of_range` after receiving, and
  `ChTable::defaults_out_of_range` for a `default` cell that violates its
  own row — that one carrying the grid row and column, so an editor
  colours the cell. Issue `value_out_of_range`, with `used` naming the
  bound crossed.

  A range is observed, never enforced: `encode` and `decode` behave the
  same whether the ask was made or not, and no answer is remembered on the
  layout. Whether the answer is wanted is a question of the moment
  (ADR-0027).
- The same three across the ABI and the .NET binding:
  `chdef_values_out_of_range`, `chdef_readings_out_of_range`,
  `Definitions.ValuesOutOfRange`, `Definitions.ReadingsOutOfRange`.

### Breaking
- **`check_capacity()` is `limits_exceeded()`.** A method that changes
  nothing is named for the state it reports, not for the act of reporting
  it (ADR-0028) — and since 0.0.8 it answers for two limits, not just the
  byte capacity. `chdef_layout_check_capacity` is
  `chdef_layout_limits_exceeded`; `Definitions.CheckCapacity` is
  `Definitions.LimitsExceeded`. One line per call site.
- `CHDEF_ABI_VERSION` is `5`.

### Changed
- ADR-0025 declined a `const` override check on the ground that what a
  caller may send is the caller's to decide. That reasoning was about
  `encode` reporting automatically and did not cover a caller asking, so
  the range asks appeared to contradict it. The ground is now the
  criterion of ADR-0027: chdef answers where a consumer cannot compute the
  answer. `min` / `max` reach a consumer in their cell's notation and need
  chdef's resolution rules; `kind` reaches it as a finished string. The
  decision itself is unchanged — there is no `const` check.

## [0.0.9] - 2026-08-25

### Fixed
- **A clamped value is no longer written in silence.** A physical value the
  channel width cannot hold reaches the wire as a different number;
  `encode` now returns Issue `encode_value_clamped`, naming the value given
  and the physical value actually written. The clamping itself is
  unchanged and still right — a reading past the end of a sensor's range
  saturates — but every other lossy reading of a cell says so
  (`bytes_out_of_range`, `raw_out_of_range`, `min_max_swapped`), and this
  one did not.

  **This can appear on traffic that has always encoded out-of-range
  values.** A consumer asserting on the count of Issues will see it; the
  `IssueCode.All` of 0.0.8 is how a table keyed by code stays complete.

### Added
- `ChannelDef::fits_width(value)`: whether the width can hold a value as
  given. The primitives (`value_to_raw`, `value_to_bytes`) keep clamping
  without an Issue, because they answer with a number and not with
  findings — this is the ask for a caller using them.
- An `E` line of a golden vector takes an optional fourth field, the
  Issues that encode produces. A value the width clamps is a different
  number on the wire, and an implementation that writes the same bytes in
  silence is not the same implementation, so this is now contracted across
  all three paths.
- A test reads the Japanese vocabulary out of `docs/spec/format.md` §2 and
  compares it with `columns.rs`, both directions. The table lived in two
  places kept by hand and nothing proved they agreed; the BF half of the
  specification is a table now rather than prose, so it can be read the
  same way.

## [0.0.8] - 2026-08-25

### Added
- **A `kind` column**: who decides a channel's value — `plain` (the
  default, and what an empty cell means), `const`, `counter`. It is a mark
  and not a behaviour: chdef carries it, exposes it on `ChannelDef`,
  `chdef_layout_channel_text` and `Channel.Kind`, writes it back, and acts
  on none of it. `encode` produces the same bytes whatever it says, and a
  `counter` is never advanced by chdef — a counter belongs to the line
  that sends the frames, and one definition may be shared by several
  (ADR-0025). An unrecognised value reads as `plain` with Issue
  `kind_assumed`.
- **`channel_capacity`**, the maximum number of channels the port accepts
  — the limit a byte count cannot express, since a 64-channel port takes
  300 two-byte channels inside any byte budget. Set with
  `with_channel_capacity` / `chdef_layout_set_channel_capacity` /
  `Definitions.ChannelCapacity`; Issue
  `layout_exceeds_channel_capacity`.
- **Every Issue code is enumerable**: `IssueCode::all()`,
  `chdef_issue_code_count` / `chdef_issue_code_name`, and an `IssueCode`
  class of constants beside `IssueCode.All` in the .NET binding. The codes
  still cross as strings so the vocabulary can grow (ADR-0021); what was
  missing was a way to ask what they are, so a consumer keying a table by
  code can prove it complete instead of finding the gap when a count moves
  (ADR-0026). The constants are not an enum: a code this assembly has not
  heard of still arrives as the string it is.

### Breaking
- `ChannelLayout::check_capacity()` answers with `Vec<Issue>` rather than
  `Option<Issue>`. A layout can be over both limits, and a consumer that
  learns one at a time fixes one at a time. The ABI and the .NET binding
  are unchanged — both already carried a list.
- The canonical CH header is 17 columns; `kind` is **appended**, so the
  9- and 10-column positional forms read exactly as before.
- `CHDEF_ABI_VERSION` is `4`.

## [0.0.7] - 2026-08-25

### Breaking
- **Japanese header spellings are no longer read by default.** A column
  has one canonical name and every other spelling belongs to a
  `ColumnVocabulary` the caller supplies; reading with none recognises the
  canonical names and their English variants alone. A file with Japanese
  headers is read by passing `ColumnVocabulary::japanese()` —
  `parse_ch_csv_with`, `ChTable::parse_with`, or `Definitions.Parse(ch, bf,
  vocabulary)` on the .NET side.
- `ColumnAliases` and `HeaderLanguage` are gone, replaced by
  `ColumnVocabulary`. `ChColumn::name(lang)` is `ChColumn::name()`, and
  `with_columns(columns, language)` is `with_columns(columns,
  &vocabulary)`.
- `CHDEF_ABI_VERSION` is `3`.

### Added
- `ColumnVocabulary`: spellings to columns for reading, and the spelling
  to write for each — the **first** taught for a column is the one
  written, so a vocabulary that reads a header can also write it.
  `ColumnVocabulary::japanese()` is one such value and has no standing a
  caller-built one lacks; `with` composes two.
- `parse_ch_csv_with`, `parse_bf_csv_with`, `parse_ch_csv_bytes_with`,
  `parse_bf_csv_bytes_with`, `load_ch_csv_with`, `load_bf_csv_with`.
- `ChColumn::variants()` / `BfColumn::variants()`: the other spellings of
  the canonical name itself, recognised with no vocabulary.
- The vocabulary crosses the C ABI and the .NET binding
  (`chdef_vocabulary_new` / `_japanese` / `_teach` / `_free`,
  `chdef_layout_parse_with`; the `ColumnVocabulary` class). A column
  crosses as its canonical **name**, so adding one to the format is not an
  ABI break — `chdef_column_count` and `chdef_column_name` report them.

### Changed
- `docs/spec/format.md` §2 is rewritten around the vocabulary, and prints
  the Japanese one as a table where it is visibly one vocabulary rather
  than half the mechanism.

### Fixed
- The ABI no longer claims to report a **freed** handle. Reading a tag out
  of memory the allocator has taken back is undefined, and the claim held
  only where the allocator happened to leave the bytes alone; a macOS
  runner showed it does not. What is contracted now is what can be kept: a
  null handle, and a handle of one kind passed where another was expected,
  are `CHDEF_ERR_HANDLE`. Using a freed handle is undefined, as for any C
  pointer.
- The opaque handles are `#[repr(C)]`, so the tag that distinguishes them
  is reliably their first field rather than wherever the compiler put it.

## [0.0.5] - 2026-08-24

### Added
- The C ABI and the .NET binding carry the named bits of a channel and of
  a decoded frame (`chdef_layout_bit_at` / `chdef_layout_bit_text` /
  `chdef_layout_bit_total` / `chdef_decode_bits`; `Channel.Bits` and
  `Reading.Bits`). A frame decodes its bits in one pass, not one call per
  bit.
- They carry the grid: a definition file as its cells, read, edited and
  written back in the shape it was read in (`chdef_grid_*`; the `Grid`
  class).
- They carry the notation of a value — `0x` is raw, anything else physical
  (`chdef_value_parse`; `Value.Parse` / `Value.TryParse`, and the
  accessors that make a parsed value readable).
- They carry which reading the `format` column selects and its default
  text form (`chdef_layout_channel_displayed` / `_render`;
  `Definitions.Displayed` / `Definitions.Render`).
- `docs/spec/abi.md` states what crosses the boundary and why, so the next
  request is answered by reading it.

### Changed
- `CHDEF_ABI_VERSION` is `2`, and the check a caller makes is that the
  library is **at least** what its declarations need. Symbols are added
  and never withdrawn, so the previous equality reading would have broken
  a correct caller on every addition.
- The golden vectors' bit-reading and diagnostics lines run through all
  three paths. Every line of every set is now checked everywhere.

### Breaking
- `Channel.BitCount` in the .NET binding is gone; the bits themselves are
  `Channel.Bits` and the count is `Bits.Count`, rather than a second
  number saying the same thing.
- `Reading` gained a `Bits` member, so positional deconstruction of the
  record takes four elements.

## [0.0.4] - 2026-08-24

### Changed
- The package descriptions on crates.io and nuget.org, and the crate's
  own front page, are English throughout. That the columns also have
  Japanese spellings is a fact about the file format, stated where the
  format is (`docs/spec/format.md` §3), not something a one-line blurb on
  a registry should be half-written in.

## [0.0.3] - 2026-08-24

The first release published to crates.io and nuget.org. `0.0.1` and
`0.0.2` below were development versions; neither was ever published, so
everything under both is in `0.0.3` too.

### Added
- A .NET binding (`bindings/dotnet/`, `net8.0`, ADR-0022): the P/Invoke
  declarations over the C ABI and a safe wrapper that owns the handles,
  does the two-call buffer dance for every string and turns a status into
  an exception. A NuGet package carries the native library per platform, so
  a consumer writes `dotnet add package` and nothing else. The golden
  vectors of `interchange.md` §3 run through it in this repository's CI —
  the same files certify the crate, the C ABI and the binding — and a Rust
  test proves the declarations mirror the ABI in name, order and width,
  which is the mismatch no vector could catch.
- `chdef-capi`, a C ABI over the crate (ADR-0021): read CH / BF
  definitions into an opaque layout handle, describe the layout, encode and
  decode frames, and read the diagnostics. It carries no enums — an Issue's
  code and a channel's `type` cross as their stable ASCII strings — every
  string it hands out goes into the caller's buffer, and every entry point
  catches panics. `crates/chdef-capi/include/chdef.h` is the header, and a
  test proves it declares every exported symbol. The golden vectors of
  `interchange.md` §3 run through the ABI as well as through the crate, so
  the boundary cannot become a second implementation.
- `Grid` (ADR-0020): a CSV file as its cells, with no column vocabulary —
  a consumer that displays or edits a definition without reading its
  columns uses one and never picks between a CH and a BF table.
  `ChTable` / `BfTable` hold a grid, forward its operations and expose it
  through `grid()`; the `grid_api!` macro that had generated those
  operations into both types is gone.
- `ChannelDef::displayed_value` / `render`: which reading the `format`
  column selects, and a default text form of it a consumer may replace
  (ADR-0015). They take a raw integer, so no byte order is assumed.
- `ChannelLayout::capacity` / `with_capacity`: the layout carries the
  capacity it is measured against, and `check_capacity()` reads it
  (ADR-0016). `Definitions::of` puts it in the JSON without restating.
- `Decoded::bits`: the named bits of a decoded channel and whether each is
  set, so a consumer displaying a bit field writes no shifts.
- `ChTable` / `BfTable`: `header`, `rows` and `row(index)` hand over the
  grid an editor draws.
- `ColumnAliases`, with `ChTable::parse_with` / `parse_bytes_with` and the
  same on `BfTable`: a reader can be taught the header spellings one
  consumer's files use (ADR-0019). An alias only ever adds a spelling,
  never reaches the writer, and never appears in the golden vectors, so
  the format and what conformance means are unchanged.
- `ChColumn`, `BfColumn` and `HeaderLanguage` are public again, and
  `ChTable::with_columns` / `BfTable::with_columns` create a table whose
  header names the columns the caller asked for, in the language asked for
  — the parameter ADR-0003 specified and no API had offered.
  `ChColumn::canonical` / `positional` / `name` / `from_header` come with
  them.
- `Issue` carries `found`, `used`, `channel` and `bit` (ADR-0018), so a
  consumer writes its own sentence in its own language without parsing
  chdef's English. `found` keeps the notation of the cell it came from,
  and the fields name what a rowless finding is about. `message` remains,
  now stated as prose whose wording is not part of the contract.
- `CsvStyle` / `LineEnding`, with `style` / `set_style`: a table writes the
  byte-order mark and record separator it read, so editing one cell of a
  file kept with LF endings no longer rewrites every line (ADR-0017). A
  file that already follows the write rules round-trips byte for byte; a
  table created in code still writes a BOM and CRLF.

### Fixed
- `parse_bytes` stripped the byte-order mark before the text was parsed, so
  a file fed as bytes was written back without the mark it had. The mark is
  valid UTF-8 and the reader already ignores it, so it is no longer
  stripped and the shape is recorded as for any other file (ADR-0020).

### Changed
- `Issue` is `#[non_exhaustive]`: it is chdef's to construct, and more
  fields may follow.
- `DisplayFormat` is `ValueDisplay { Physical, Raw }`, naming the choice
  the `format` column makes rather than the base its cell is spelled in
  (ADR-0015). `parse` still reads `DEC` / `HEX` and `as_str` writes them.
- Issue `hex_with_lsb` is `raw_display_with_lsb`, and the definitions JSON
  spells `format` as `"physical"` / `"raw"`.
- `check_capacity` takes no argument and reads the layout's capacity.

### Removed
- `ChannelDef::format_value`, which rendered with the byte order hardcoded
  to little-endian. `render` replaces it.

## [0.0.2] - 2026-08-24

### Added
- `chdef` crate: `parse_ch_csv` / `parse_bf_csv` / `load_ch_csv` /
  `load_bf_csv` / `build_layout` / `ChannelDef::raw_to_value_endian`, moved
  from `channel.rs` / `csv_loader.rs` of `chbridge-core` without changing
  behaviour. `Endian` and `ChdefError` are the crate's own types.
- Specification (`docs/spec/`) and ADR-0001 / ADR-0002 (`docs/decisions/`).
- `ChannelDef::value_to_raw` / `value_to_bytes` / `value_to_bytes_endian`:
  physical → raw (half away from zero, clamped to the channel width, two's
  complement, 1–8 bytes) and `ChannelDef::bits`.
- `ChColumn` / `BfColumn` / `ColumnMap` / `HeaderLanguage`: columns are
  identified by header name in English (`number,bytes,…`) or Japanese
  (`番号,バイト数,…`), case-insensitively; a first row without a `number`
  column is data in canonical order (ADR-0003).
- `parse_ch_csv_bytes` / `parse_bf_csv_bytes`: parse a CH / BF CSV from bytes
  for a consumer that holds no path (a file dialog returning a stream, a
  browser file input). They drop leading BOMs and decode as UTF-8, failing
  with `ChdefError::Encoding { valid_up_to }` on anything else (ADR-0004).
- `Issue` / `IssueCode` / `Parsed`: a problem in one row no longer stops
  loading — every readable row is read and the problem comes back as an
  `Issue { code, row, col, message }` next to the value
  (`docs/spec/diagnostics.md`). 16 of the 19 specified codes are emitted;
  `bf_bit_out_of_range`, `bf_parent_not_bitfield` and
  `layout_exceeds_capacity` need cross-file input and are still open.
- Blank rows and rows whose first cell starts with `#` are skipped without
  an Issue.
- `ChannelDef::new` / `BitFieldDef::new`: construct a definition with its
  identity; every other field starts at its unspecified value and is set
  directly (ADR-0005).
- `serde` became an opt-in feature; the default build no longer depends on
  it (ADR-0005).
- `min` / `max` are interpreted: physical bounds, or raw bit patterns with
  the `0x` prefix (`Bound`), carried on `ChannelDef` and never applied by a
  conversion; `min_value` / `max_value` / `range_contains` /
  `clamp_to_range` are the explicit queries, and `min_invalid` /
  `max_invalid` / `min_max_swapped` the new Issues (ADR-0006).
- `ChannelLayout::endian`: the whole-layout byte order of `layout.md` §2,
  set by the consumer (`Little` when unset); frame encode / decode will
  consume it.
- `raw_to_bytes_endian` / `raw_from_bytes_endian`: the storey below the
  physical conversion — a raw bit pattern to / from the channel's bytes,
  truncating to the width with no rounding and no clamp (ADR-0007).
- `ChannelLayout::decode` / `channel_bytes` (`conversion.md` §6): slice a
  frame into per-channel bytes with raw and physical readings under the
  layout's `endian`; a channel that overruns a short frame is omitted.
- Golden vector sets `widths`, `scaling`, `bitfields` and `diagnostics`,
  and the `B` (byte order), `F` (BF bit values) and `P` (expected Issues)
  lines the last three needed. The contract now covers all eight legal
  widths, both byte orders, non-zero `lsb` / `offset`, the BF default
  merge, and the Issues a broken definition set produces — the areas the
  first set was blind to, one of which hid the width defect above.
- Golden vectors (`crates/chdef/vectors/`, `interchange.md` §3, ADR-0013):
  the cross-language contract as `ch.csv` / `bf.csv` / `vectors.txt` per set,
  shipped inside the package, with a harness that runs every set and names
  the vector file and line on a mismatch.
- `interchange` module behind the `serde` feature (`interchange.md` §1 / §2,
  ADR-0013): `Definitions::of(&layout, &issues)` (with `with_capacity`),
  `Readings::of(&decoded)` and `ChTable::to_json` / `BfTable::to_json` build
  the documented JSON shapes as their own types, separate from the domain
  types so the wire format and the definitions can grow independently. The
  consumer picks the serializer; chdef depends on none.
- `Value` implements `Display`, writing exactly what `Value::parse` reads.
- `ChannelDef` carries `section` / `memo` / `var` / `format` / `favorite`
  and `BitFieldDef` carries `memo`, so a consumer no longer re-scans the
  cells to recover columns chdef already read. `DisplayFormat` (`DEC` /
  `HEX`) is a type with its own parse; it never affects a conversion.
- `ChannelDef::raw_to_value_u64`: the physical conversion for a raw value
  already held as an integer, without the byte round trip.
- `ChannelLayout::positions`: every channel with its byte offset, in row
  order — the walk `encode` and `decode` perform.
- `BfTable::cross_issues` (ADR-0012): the layout's cross-file BF checks,
  run on the grid where rows still exist — each finding carries the row
  and the `number` / `bit` column, so an editor can point at the cell.
- `ChannelLayout::encode` / `channel_default` (`conversion.md` §5 / §4,
  ADR-0011): build a frame from per-channel `Value`s — physical converted
  and clamped, raw truncated, unnamed channels filled with their default
  with BF bits folded in. Unknown numbers and non-finite values are
  reported (`encode_unknown_channel` / `encode_value_invalid`), never
  dropped. `Value::parse` reads the `0x`-raw / plain-physical notation for
  consumer input.
- `ChTable` / `BfTable` (`docs/spec/editing.md`, ADR-0009): the Table stage
  as a verbatim cell grid — unknown columns, comment rows and header
  spelling survive read → edit → `to_csv` at cell granularity. Cell / row
  edits, `insert_channel` (typed insertion into the columns the file has),
  and `insert_channel_renumbering`, which shifts later numbers with their
  BF parents and reports every `(old, new)` pair as `Renumbered`
  (ADR-0010). `parse_ch_csv` / `parse_bf_csv` are reimplemented on the
  Table with unchanged behaviour.
- `build_layout` runs the cross-file BF checks and returns
  `Parsed<ChannelLayout>` (`bf_parent_not_bitfield` / `bf_bit_out_of_range`,
  without rows — ADR-0008); `ChannelLayout::check_capacity` reports
  `layout_exceeds_capacity`. Every code of the diagnostics spec is now
  emitted. `BitFieldDef::bit_of` extracts a bit from the parent's raw value.

### Fixed
- An unterminated quote read the rest of the file into one cell, so the
  later rows vanished with no Issue and no error — the exact loss
  `format.md` §1 warns about, and the error `diagnostics.md` §1 promises.
  `ChdefError::CsvParse` was unreachable; every entry point now refuses
  such a file and names the line the quote opened on. Its `row` field
  became `line`, saying which base it counts in.
- `default` was capped at 32 bits, so a channel wider than 4 bytes could
  not state one and got `default_invalid` — an Issue whose message denied
  that a well-formed `0x` value was one. A default is now as wide as its
  channel, and a value past that width is `raw_out_of_range` with the low
  bits kept, in decimal as well as hexadecimal, matching the verdict the
  same text already got in `min` / `max`.
- A BF `bit` of 64 or more was reported as `bf_bit_invalid` ("not an
  integer") instead of `bf_bit_out_of_range`.
- `insert_channel` left `favorite` empty instead of writing `0`.
- `encode` truncated a raw value past the channel width without reporting
  it.
- `remove_row` panicked on an index past the end; it returns `None`.
- `Renumbered.moved` repeated a pair once per row rather than once per
  channel, and renumbering a channel at `u32::MAX` overflowed.
- Conversions used two different widths: `raw_to_value_endian` measured a
  channel by its `DataType` while everything else measured it by
  `byte_count`, so a 3-, 5-, 6-, 7- or 8-byte channel read only its first
  two bytes. A 3-byte `SI` channel round-tripped `−100 000` as `+31 072`,
  and `ChannelLayout::decode` returned a `raw` and a `value` that
  disagreed inside one `Decoded` (ADR-0014).
- `raw_to_value_u64` did not sign-extend a 64-bit `SI` channel, so `−1`
  read back as `1.8e19` and every range query built on `min_value` /
  `max_value` inherited it.
- `value_to_raw` clamped in f64, where `2^n − 1` is not representable
  beyond 53 bits; a 7-byte unsigned channel clamped to `0` instead of its
  maximum.
- A `byte_count` of 0 made `bits()` zero and overflowed a subtraction in
  `raw_to_value_u64`.
- `BF` channels read big-endian were shifted, because the byte reader
  zero-padded at the tail regardless of byte order.

### Changed
- `DataType` is `UI` / `SI` / `BF` — the interpretation only (ADR-0014,
  `layout.md` §6). `byte_count()` and `resolve()` are gone; `as_str()` and
  `Display` give the two-letter tag. `ChannelDef::width()` is the single
  authority for how wide a channel is, holding `byte_count` to 1–8.
- `load_ch_csv` / `load_bf_csv` take `impl AsRef<Path>` instead of `&str`, so
  a `PathBuf` and a path that is not valid Unicode both go through. Calls
  passing a string literal are unaffected.
- `parse_ch_csv` / `parse_bf_csv` / their `_bytes` and `load_` forms return
  `Parsed<Vec<…>>` (value plus Issues) instead of a bare `Vec`, and keep
  duplicate rows; `build_layout` now drops duplicates first-wins.
- `BitFieldDef::default_value` is `Option<u8>`: an empty or invalid BF
  `default` is unspecified (the parent channel's bit is kept), no longer 0.
- `ChannelDef::lsb` is stored resolved: an empty, `0`, or invalid `lsb`
  arrives as `1.0` instead of `0.0`.
- `Bound` is renamed `Value` (ADR-0011): the same notation-carrying pair
  now feeds `min` / `max`, form input and encode. `ChannelDef::min` /
  `max` semantics are unchanged.
- `ChannelLayout::total_bytes` is a method computed on demand instead of a
  stored field, so an edited `byte_count` can no longer leave it stale
  (ADR-0006).
- `ChdefError` is `#[non_exhaustive]` like the other growing vocabularies
  (ADR-0005): external matches need a catch-all arm.
- The public surface is the crate root only (ADR-0005): the `channel` /
  `columns` / `csv` / `error` / `issue` module paths are private, and
  `ChColumn` / `BfColumn` / `ColumnMap` / `HeaderLanguage` are withdrawn
  until the writer exists. `ChannelDef`, `BitFieldDef`, `ChannelLayout` and
  `DataType` are `#[non_exhaustive]`, so the fields and variants the
  specification already promises can arrive without breaking callers.
- `parse_ch_csv` / `parse_bf_csv` locate every column by header name instead
  of position; the default column matches exact spellings (`default`,
  `値(デフォルト)`, `デフォルト値`, `DefaultValue`) rather than any header
  containing "デフォルト".
