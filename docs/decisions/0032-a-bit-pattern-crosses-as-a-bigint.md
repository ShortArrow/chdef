# ADR-0032: A bit pattern crosses to JavaScript as a bigint

- Status: Accepted
- Date: 2026-08-25
- Release: 0.0.15

## Context

A JavaScript `number` is a double, so it holds integers exactly only up
to 2^53 - 1. A chdef channel may be eight bytes wide, and a raw value is
the bit pattern of that channel read as an integer — up to 2^64 - 1.

The first WebAssembly binding carried every number as a `number`. The
golden vectors found the consequence on the line

    E 1=7;2=0x0102030405060708;3=9 0700080706050403020109000000

which encoded as `070000` where it should have encoded `070008`. The raw
value `0x0102030405060708` had been rounded to `0x0102030405060700` on
the way in, and the low byte of the frame was silently wrong. Nothing
else in the binding reported anything: the frame was the right length,
the Issue list was empty, and only the vector's expected bytes disagreed.

Against `bigint`: it is contagious in arithmetic — mixing it with a
`number` throws rather than coercing — and `JSON.stringify` refuses it.
Both costs fall on the common case of a small channel, where a `number`
would have been exact.

## Decision

- **A raw bit pattern crosses as a `bigint`**: `Reading.raw`, a
  channel's `default`, the `bits` of a raw `Value`, and the `raw`
  argument of `displayed` and `render`.
- **A physical value crosses as a `number`.** It is an `f64` in the
  crate, so `number` carries it exactly; making it a `bigint` would lose
  the fraction that `lsb` exists to produce.
- **A `Value` names which of the two it is** with a `form` field rather
  than by which field is filled, so neither a missing field nor both at
  once can be written:

      type Value =
        | { form: "physical"; channel: number; value: number }
        | { form: "raw"; channel: number; bits: bigint };

## Consequences

An eight-byte channel round-trips exactly through JavaScript, which is
the only reason to have a binding rather than a reimplementation.

A record holding a bit pattern cannot be passed to `JSON.stringify`
without a replacer. `structuredClone` carries it, so a worker and
IndexedDB are unaffected; only text serialization needs the caller's
attention, and the README says so.

A caller doing arithmetic on a raw value works in `bigint` or converts
with `Number()`, accepting the rounding explicitly at the point they
choose it rather than silently at the boundary.
