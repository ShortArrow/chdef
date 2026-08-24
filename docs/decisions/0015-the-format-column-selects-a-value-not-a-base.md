# ADR-0015: The `format` column selects which value is shown, not the base it is printed in

- Status: Accepted
- Date: 2026-08-24
- Release: 0.0.3

## Context

The `format` column is spelled `DEC` / `HEX` in the files, and the crate
mirrored that spelling into `DisplayFormat { Dec, Hex }` with an Issue
called `hex_with_lsb`. Reading the specification back, the names were
naming the wrong thing three times over:

- `docs/spec/conversion.md` §7 defines the column as "The consumer shows
  the **raw value** when the format is `HEX`" — by which reading is
  shown, not by the base it is printed in.
- `hex_with_lsb` only earns its place under that reading. If `HEX` were
  merely a base there would be nothing wrong with printing a physical
  value in it; the Issue exists because `HEX` means "show the raw", and
  the raw is not the physical quantity once `lsb` is not 1.
- `docs/spec/README.md` puts "Presentation (DEC / HEX rendering, digit
  counts, colours)" out of scope, while the type was named after exactly
  that rendering.

Separately, `ChannelDef::format_value` rendered a reading with the byte
order hardcoded to little-endian, so a consumer with a big-endian layout
got byte-swapped text — and three consumers had written their own
rendering rather than use it.

## Decision

- **`DisplayFormat` becomes `ValueDisplay { Physical, Raw }`**, naming
  the choice the column makes. `parse` still reads `DEC` / `HEX` and
  `as_str` still writes them: the file spelling is frozen by files that
  exist, and mapping it belongs in one place.
- **`hex_with_lsb` becomes `raw_display_with_lsb`**, and its message says
  what is wrong rather than which keyword was used.
- **The JSON says `"physical"` / `"raw"`.** The definitions JSON is the
  interpreted view — the verbatim cell is what the Table JSON carries —
  so it states the meaning.
- **`format_value` is replaced** by `displayed_value(raw) -> Value`,
  which answers only the question that is chdef's (which reading), and
  `render(raw) -> String`, a default text form the consumer may take or
  replace. Both take a raw integer, so no byte order is assumed.

## Alternatives rejected

- **Renaming the CSV cell values too**: existing files spell them `DEC` /
  `HEX`; the spelling is data, not a name chdef gets to choose.
- **Keeping `DisplayFormat` and renaming only the variants**: leaves the
  type named after the base while its variants name the value.
- **Dropping rendering entirely** to honour the out-of-scope line: three
  consumers had already rebuilt it, and a bypassed abstraction is a
  discarded one. Offering a replaceable default is the middle the scope
  line now states.

## Consequences

- `ChannelDef::format` keeps its name — it is the `format` column — while
  its type says what the column means.
- Consumers that had a `ChannelText` equivalent can delete it, or keep
  their own digits and take `displayed_value` alone.
- The byte-swapped rendering of a big-endian layout is gone with
  `format_value`.
