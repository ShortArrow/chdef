# Conversion

🌐 **English** | [日本語](./conversion.jp.md)

Implemented (0.0.2): raw → physical (`raw_to_value_endian`; UI / SI 8–32 bit
and BF, LSB 0 → 1, offset, endian) and physical → raw (`value_to_raw` /
`value_to_bytes_endian`; 1–8 bytes, half away from zero, clamp, two's
complement), and the range queries of §8. 64-bit decode, BF default
merging, and encode / decode of whole frames are not implemented yet.

## 1. Raw → physical

```
value = raw_signed × lsb + offset
```

- `raw_signed`: the raw value of width `bytes × 8` bits, sign-extended as
  two's complement for `SI`, read as unsigned for `UI` / `BF`.
- `lsb`: `lsb` from the CSV (0 / empty is 1). `offset`: `offset` from the
  CSV (empty is 0).
- An 8-byte (64-bit) raw value may lose precision in f64 (integers beyond
  53 bits). The physical value is returned as f64; the raw value is returned
  separately as a 64-bit integer.

## 2. Physical → raw

```
raw = clamp(round((value − offset) ÷ lsb))
```

- `round` rounds **half away from zero** (0.5 → 1, −0.5 → −1, 2.5 → 3).
- `clamp`: `SI` to `[−2^(bits−1), 2^(bits−1) − 1]`, `UI` / `BF` to
  `[0, 2^bits − 1]`. After clamping, a negative value becomes its
  two's-complement bit pattern.
- A NaN / infinite `value` cannot be converted (`None`).

## 3. Raw values given directly

- A string with the `0x` prefix is read as a raw value (LSB / offset are not
  applied). Bits beyond the width are reported as Issue `raw_out_of_range`
  and only the low bits are used.

## 4. Defaults

- A channel's default is `default` (a raw value; 0 when unspecified).
- For a channel whose `type` is `BF`, each BF CSV row overrides one bit:
  `1` sets the bit, `0` clears it, unspecified keeps the channel default's
  bit. With no BF rows the channel default stands.

Example: channel default `0x0010`, BF `{BIT0=1, BIT2=1, BIT4=unspecified}`
→ `0x0015`. Channel default `0x00FF`, BF `{BIT0=0, BIT4=0}` → `0x00EE`.

## 5. encode (values → frame)

- Input: a physical value, or a raw value (`0x`), per channel. Channels not
  given use their default.
- Output: a byte string of `total_bytes`. Each channel's raw value is written
  at `at` for `bytes` bytes according to `endian`.
- Values beyond the width are clamped as in §2; raw values keep the low bits
  as in §3.

## 6. decode (frame → values)

- Input: a byte string.
- Output: raw and physical value per channel. If the frame is short and a
  channel **overruns it, that channel is omitted from the result** (not
  zero-filled, so a value that did not arrive never looks as if it did).
- A BF bit is `(raw >> bit) & 1` of the parent's raw value.

## 7. Display format

- `format` does not affect chdef's conversion (`value` is returned even
  for `HEX`). The consumer shows the raw value when the format is `HEX`.
  `HEX` with `lsb ≠ 1` is Issue `hex_with_lsb` ([format.md](./format.md)).

## 8. Range (min / max)

- A `min` / `max` bound is a physical value; with the `0x` prefix it is a
  raw bit pattern, resolved with the channel's **current** `lsb` / `offset`
  when queried (sign-extended for `SI`), so an edited `lsb` moves it.
- No conversion applies the range. The caller opts in explicitly:
  `range_contains` answers whether a physical value lies inside it, and
  `clamp_to_range` clamps one into it. An unspecified side is unbounded;
  NaN is never inside; a swapped range matches nothing.
