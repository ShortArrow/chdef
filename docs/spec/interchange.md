# Interchange formats

🌐 **English** | [日本語](./interchange.jp.md)

Implemented (0.0.2): the JSON shapes of §1 and §2 behind the `serde`
feature (`interchange::Definitions` / `Readings` / `ChTable::to_json`;
chdef builds the value, the consumer serialises it), and the golden
vectors of §3 with the harness that runs them. TypeScript type generation
is not implemented.

## 1. JSON

A consumer that receives definitions over the wire — a browser UI, a GUI
across a socket — reads this JSON rather than the CSV. The shape is fixed
as follows (keys are stable; additions are backward compatible). A
consumer that reads the CSV itself instead conforms to the golden vectors
of §3.

```json
{
  "total_bytes": 13,
  "capacity": 246,
  "endian": "little",
  "channels": [
    {"n": 1, "at": 0, "bytes": 4, "type": "UI", "section": "General", "name": "Frame counter",
     "lsb": 1.0, "offset": 0.0, "unit": "", "default": null,
     "format": "physical", "min": "", "max": "", "memo": "", "var": "", "favorite": false}
  ],
  "bitfields": [
    {"n": 2, "bit": 0, "name": "Reserved", "default": null, "memo": ""}
  ],
  "issues": [
    {"code": "type_assumed", "row": 3, "col": 5, "channel": 4, "bit": null,
     "found": "ZZ", "used": "UI", "message": "..."}
  ]
}
```

- `lsb` is `1.0` even when the CSV has 0 / empty (the reader carries no
  rule).
- `default` is `null` when unspecified; the same for a BF `default`.
- `format` says which reading the channel shows — `"physical"` or
  `"raw"` — the meaning of the `DEC` / `HEX` cell, since this is the
  interpreted view. The verbatim cell is in the Table JSON of §2.
- `capacity` is present only when the layout carries one, or one was
  passed.
- Value JSON (decode result): an array of `{"n": 4, "raw": 65413, "value": -12.3}`.
  A physical value that is not a finite number — NaN or ±∞ — is `null`,
  which is all JSON can say about it.

## 2. Table (cells) JSON

```json
{"header": ["番号", "バイト数", "..."], "rows": [["1", "4", "..."], ["2", "2", "..."]]}
```

For editing UIs. Verbatim, including unknown columns — the grid's shape,
so a consumer that reads a file only as cells produces it without
choosing between a CH and a BF table.

## 3. Golden vectors

The cross-language contract. Each set lives in
`crates/chdef/vectors/<name>/` as `ch.csv` / `bf.csv` / `vectors.txt` — inside
the package, so the published crate carries them — and the tests of every
language read the same files. No real-device definitions (all synthetic).

Format of `vectors.txt` (`#` is a comment, blank lines are ignored):

```
# B <little|big>                   : byte order of the lines that follow (little until said otherwise)
# E <n=value;...> <wire hex>       : frame encoded from the values. Unlisted channels use their default, else 0. '-' means all defaults
# D <wire hex>  <n=raw/value;...>  : raw and physical values decoded from the frame. A short frame drops overrunning channels
# F <wire hex>  <n:bit=0|1;...>    : the value of named bits inside a decoded BF channel
# L <total_bytes> <n:at:bytes;...> : layout
# P <ch|bf|layout> <code:row;...>  : the Issues that source produces. '-' for none, and '-' as the row for an Issue that carries none
L 13 1:0:4;2:4:2;3:6:1;4:7:2;5:9:4
B little
E 1=1;2=5;3=2;4=-12.3;5=1.5 0100000005000285ffdc050000
E - 00000000010000000000000000
D 0100000005000285ffdc050000 1=1/1.0;2=5/5.0;3=2/2.0;4=65413/-12.3;5=1500/1.5
D 0100000005000285ff 1=1/1.0;2=5/5.0;3=2/2.0;4=65413/-12.3
F 00000000010000000000000000 2:0=1
P ch -
```

- Values in `E` are physical values; with the `0x` prefix they are raw —
  the notation of [format.md §3](./format.md#3-ch-csv), read by
  `Value::parse`.
- Physical values compare within 1e-9 relative to the expected magnitude,
  so a wide channel is compared at its own scale.
- A `P` line states which Issues a source produces and how many of each; a
  repeated Issue is not deduplicated ([diagnostics.md §1](./diagnostics.md)),
  but the order they arrive in is unspecified and is not contracted. A
  source with no `P` line must produce no Issues.
- Every vector set has at least one `E`, one `D` and one `L` line.

The sets in this repository: `basic` (the example above), `widths` (all
eight legal widths at both byte orders, at the boundaries
[conversion.md §2](./conversion.md) clamps to), `scaling` (non-zero `lsb`
and `offset` on every channel, so the terms of the core formula are
exercised), `bitfields` (the default merging of conversion.md §4 and the
named bits of §6) and `diagnostics` (a definition set that is wrong on
purpose, contracted by its `P` lines).
