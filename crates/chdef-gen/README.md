# chdef-gen

A CH / BF definition in, a constant table out: Rust source declaring a
`chdef_core::Layout`, or a C header declaring the same tables and a
`CHDEF_LAYOUT`. The firmware build runs it; the device never sees a CSV. A Rust crate
that would rather not carry the generated file can expand the same
items in place with [`chdef-macros`](../chdef-macros/README.md).

    chdef-gen --ch ch.csv [--bf bf.csv] [--endian little|big] [--japanese] \
        [--rust layout.rs] [--c layout.h]

At least one of `--rust` and `--c` is needed. The files are read the way
the host reads them, `--japanese` included, so a definition means here
what it means there.

## A definition with any Issue is refused

Every finding goes to stderr as `chdef` reports it, naming the row that
has to be fixed, and nothing is written. A row the host would load with a
warning does not reach a device, where nothing can warn.

## What the table carries

Per channel: its number, its offset, its width, and its default with the
BF rows merged in. Per derived channel: which slot it fills, the six CRC
numbers, and the byte ranges its recipe covers, resolved from channel
numbers to offsets while the definition was still at hand.

Not carried: names, units, `lsb`, `offset`, `min`, `max`, `section`,
`memo` — everything a physical value or a diagnostic is made of. The
target does not use them and has no allocator to hold them. The channel
numbers do come along, as `CH_…` and `CHDEF_CH_…` constants, so the
firmware can name a channel rather than count it.

## Where the rules live

The table is data; the arithmetic that reads it is
[`chdef-core`](../chdef-core/README.md), which a device links for its own
target.
