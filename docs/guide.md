# Guide

🌐 **English** | [日本語](./guide.jp.md)

The shortest path through each task. What the format *is* lives in
[the specification](./spec/README.md); this page is about getting something
done.

Every call named here exists in Rust, across the C ABI, and in the .NET
and JavaScript bindings, spelled the way each language spells things.
The examples are Rust; each binding's own front page shows its spelling.
A reader writing firmware takes none of the four: a device holds the
layout, not the file, and [spec/embedded.md](./spec/embedded.md) says what
it holds instead.

## The shape of it

```
CSV bytes ──▶ Table (cells) ──▶ Rows (channels) ──▶ Layout (positions) ──▶ frames
              edit here          interpret here      encode / decode here
```

Nothing is cached. A layout is a value: edit the cells, interpret again,
and every position recomputes. `encode` and `decode` take `&self` and hold
nothing, so a layout can be shared across threads and across lines.

## Reading a definition set

```rust
let channels = chdef::parse_ch_csv(ch_csv)?;
let bitfields = chdef::parse_bf_csv(bf_csv)?;
let layout = chdef::build_layout(channels.value, bitfields.value);
```

Each of those returns `Parsed { value, issues }`. **A problem in one row
never stops the load** — it comes back in `issues`, pointing at the row and
column it is about, and the value is there beside it. Only an unreadable
*file* is an `Err`: bad encoding, an unterminated quote.

A consumer that ignores `issues` gets a definition set that loaded as well
as it could. A consumer that shows them can point at the cell.

If the headers are spelled some other way, teach them once:

```rust
let vocabulary = chdef::ColumnVocabulary::japanese();
let channels = chdef::parse_ch_csv_with(ch_csv, &vocabulary)?;
```

## Sending a frame

```rust
let mut frame = layout.encode(&[(2, chdef::Value::Physical(120.0))]).value;
layout.seal(&mut frame);
```

- Channels you do not name take their `default`; a `BF` channel's default
  has each bit's own default folded in.
- **Give a counter its value yourself, already wrapped.** chdef never
  advances one: a counter belongs to the line that sends the frames, and
  one definition may be shared by several. Pass `Value::Raw(n & mask)` —
  a physical value beyond the width saturates instead of wrapping.
- **`seal` fills the derived channels**, the CRCs. `encode` never does, so
  it stays a pure function of what you handed it. Seal once, after
  everything else is in place.
- A value the width cannot hold is clamped **and reported**
  (`encode_value_clamped`): the number on the wire is not the number you
  asked for, so chdef says so.

## Receiving a frame

```rust
for reading in layout.decode(&frame) {
    println!("{} = {} {}", reading.channel.name, reading.value, reading.channel.unit);
    for (bit, set) in reading.bits() {
        println!("  {} = {}", bit.name, set);
    }
}
```

- A channel that overruns a short frame is dropped, and so is everything
  after it — never zero-filled.
- `derived_mismatches(&frame)` is the check a receiver makes: it names
  every derived channel whose stored value disagrees with its recipe. This
  is the one place chdef says a frame is *wrong* rather than merely
  unusual.
- `readings_out_of_range(&readings)` names the readings outside their
  declared `min` / `max`. Nothing applies a range on its own.

## Asking questions

None of these changes anything, and none is remembered on the layout.
Whether you want the answer is a question of the moment.

| Ask | Answers |
|---|---|
| `limits_exceeded()` | the layout against the byte and channel limits you stated |
| `values_out_of_range(&values)` | values you are about to send, against their ranges |
| `readings_out_of_range(&readings)` | a frame that arrived, the same way |
| `ChTable::defaults_out_of_range()` | a `default` cell violating its own row — **with the grid row and column**, so an editor colours the cell |
| `derived_mismatches(&frame)` | derived channels against their recipes |
| `fits_width(value)` | whether a single value fits, before sending |

## Editing a definition file

```rust
let mut table = chdef::ChTable::parse(text)?;
table.set_cell(0, 1, "4");
let written = table.to_csv();
```

- A `Grid` is the file as its cells — comment rows, blank rows and unknown
  columns included — and it writes back in the shape it was read, byte for
  byte when the file already follows the write rules.
- `insert_channel_renumbering` keeps the numbering consecutive and returns
  `Renumbered { moved }`, the `(old, new)` pairs. Repairing references
  outside the two files is the consumer's; `moved` is what that takes.
- Facts that belong to a channel belong in a cell, not in a constant
  elsewhere: `kind` says who fills it, `derived` says how a CRC is
  computed. A cell moves with its row, and a constant does not.

## When chdef will not do it for you

Deliberately, each for a reason worth knowing:

| | why |
|---|---|
| Advancing a counter | the count belongs to the line, and one definition may serve several |
| Applying `min` / `max` | a value outside a declared range is written as given; nothing is hidden, so there is nothing to confess |
| Guessing what a CRC covers | it is a property of the protocol, and a wrong guess shows up as hardware discarding frames in silence |
| Computing a checksum it does not know | but `covered_bytes` hands you exactly the bytes it covers, so you compute your own over the right span |
| Choosing a byte order | not written in the CSV; the consumer sets it |
| Reading a header spelling it was not taught | a vocabulary is data you supply |

## Diagnostics

An `Issue` carries a stable `code`, the `row` and `col` it points at, the
`channel` and `bit` when it has them, the value it could not use
(`found`), and what it used instead or the bound it crossed (`used`).
`message` is English prose and its **wording is not contracted** — write
your own sentence from the fields.

`IssueCode::all()` lists every code, so a table keyed by code can be
proved complete at build time rather than found short at run time.

The codes and what each means are in
[docs/spec/diagnostics.md](./spec/diagnostics.md).
