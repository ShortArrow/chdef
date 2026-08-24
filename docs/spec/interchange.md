# Interchange formats

🌐 **English** | [日本語](./interchange.jp.md)

Implemented (0.0.2): the JSON shapes of §1 and §2 behind the `serde`
feature (`interchange::Definitions` / `Readings` / `ChTable::to_json`;
chdef builds the value, the consumer serialises it), and the golden
vectors of §3 with the harness that runs them. TypeScript type generation
is not implemented.

## 1. JSON

Consumers outside Rust only display and edit chdef's JSON; they never
interpret the CSV themselves. The shape is fixed as follows (keys are
stable; additions are backward compatible).

```json
{
  "total_bytes": 13,
  "capacity": 246,
  "endian": "little",
  "channels": [
    {"n": 1, "at": 0, "bytes": 4, "type": "UI", "section": "General", "name": "Frame counter",
     "lsb": 1.0, "offset": 0.0, "unit": "", "default": null,
     "format": "DEC", "min": "", "max": "", "memo": "", "var": "", "favorite": false}
  ],
  "bitfields": [
    {"n": 2, "bit": 0, "name": "Reserved", "default": null, "memo": ""}
  ],
  "issues": [
    {"code": "type_assumed", "row": 3, "col": 5, "message": "type must be UI, SI or BF; assuming UI"}
  ]
}
```

- `lsb` is `1.0` even when the CSV has 0 / empty (the reader carries no
  rule).
- `default` is `null` when unspecified; the same for a BF `default`.
- `capacity` is present only when one was passed.
- Value JSON (decode result): an array of `{"n": 4, "raw": 65413, "value": -12.3}`.
  NaN is `null`.

TypeScript types are generated from the Rust types and shipped.

## 2. Table (cells) JSON

```json
{"header": ["番号", "バイト数", "..."], "rows": [["1", "4", "..."], ["2", "2", "..."]]}
```

For editing UIs. Verbatim, including unknown columns.

## 3. Golden vectors

The cross-language contract. Each set lives in
`crates/chdef/vectors/<name>/` as `ch.csv` / `bf.csv` / `vectors.txt` — inside
the package, so the published crate carries them — and the tests of every
language read the same files. No real-device definitions (all synthetic).
A set's own definitions must load without Issues.

Format of `vectors.txt` (`#` is a comment, blank lines are ignored):

```
# E <n=value;...> <wire hex>       : frame encoded from the values. Unlisted channels use their default, else 0. '-' means all defaults
# D <wire hex>  <n=raw/value;...>  : raw and physical values decoded from the frame. A short frame drops overrunning channels
# L <total_bytes> <n:at:bytes;...> : layout
E 1=1;2=5;3=2;4=-12.3;5=1.5 0100000005000285ffdc050000
E - 00000000010000000000000000
D 0100000005000285ffdc050000 1=1/1.0;2=5/5.0;3=2/2.0;4=65413/-12.3;5=1500/1.5
D 0100000005000285ff 1=1/1.0;2=5/5.0;3=2/2.0;4=65413/-12.3
L 13 1:0:4;2:4:2;3:6:1;4:7:2;5:9:4
```

- Values in `E` are physical values; with the `0x` prefix they are raw —
  the notation of [format.md §3](./format.md#3-ch-csv), read by
  `Value::parse`.
- Physical values compare with a tolerance of 1e-9.
- Every vector set has at least one `E`, one `D` and one `L` line.
