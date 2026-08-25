# chdef

Channel definitions (CH) and bit-field definitions (BF) for binary frames:
read the CSVs, compute the frame layout, encode and decode frames.

A WebAssembly binding over the
[chdef](https://github.com/ShortArrow/chdef) Rust crate, with TypeScript
declarations generated from it — `npm install chdef` and nothing else.
The package ships a `bundler` build for Vite and friends and a `nodejs`
build for Node, and picks between them itself.

> **⚠ Pre-alpha (0.0.x).** The API and the CSV rules are still moving;
> until 0.1.0 lands, patch releases may break either.

**Nothing here throws for a bad row.** A problem comes back as an `Issue`
beside the value it is about, so a file that is wrong in one cell still
loads. An unreadable *file* — bad encoding, an unterminated quote — throws,
and so does a value whose shape is not one this page describes.

Every example below is executed by `test/readme.test.mjs` — this page is
the test, not a copy of one — and the build fails if any of them stops
holding.

## Reading a frame

```js
const ch = "number,bytes,type,name,lsb,offset,unit\n"
         + "1,2,UI,speed,0.5,0,km/h\n"
         + "2,1,BF,status,1,0,\n";
const bf = "number,bit,name\n2,0,ready\n2,1,fault\n";

const defs = Definitions.parse(ch, bf);
assert.deepEqual(defs.issues, []);

// 0x0040 little-endian is raw 64, which is 32 km/h at lsb 0.5.
const readings = defs.decode(Uint8Array.from([0x40, 0x00, 0b01]));

assert.equal(readings[0].value, 32);
assert.deepEqual(readings[1].bits.map((b) => [b.name, b.value]),
                 [["ready", true], ["fault", false]]);
```

## Building one

`encode` writes the values you give and the defaults of the channels you
do not. A channel the definitions mark as `derived` — a CRC — is filled by
`seal`, which is a call of its own so that `encode` stays a pure function
of what you handed it.

```js
const ch = "number,bytes,type,kind,derived,default,name\n"
         + "1,2,UI,const,,0x7E7E,sync\n"
         + "2,2,UI,plain,,,speed\n"
         + "3,2,UI,derived,crc16/x25 1..2,,crc\n";

const defs = Definitions.parse(ch);
const encoded = defs.encode([{ form: "physical", channel: 2, value: 120 }]);

assert.deepEqual(encoded.issues, []);
assert.deepEqual([...encoded.frame.slice(0, 2)], [0x7E, 0x7E]);
assert.deepEqual([...encoded.frame.slice(4)], [0, 0]);

const sealed = defs.seal(encoded.frame);
assert.deepEqual(sealed.issues, []);
assert.notDeepEqual([...sealed.frame.slice(4)], [0, 0]);
assert.deepEqual(defs.derivedMismatches(sealed.frame), []);
```

## Numbers

A physical value is a `number`. **A raw bit pattern is a `bigint`**,
because a channel may be eight bytes wide and a `number` holds only 53
bits of integer exactly. `structuredClone` carries a `bigint`, so a
reading may be posted to a worker; `JSON.stringify` does not, so text
serialization needs a replacer.

```js
const defs = Definitions.parse("number,bytes,type,name\n1,8,UI,counter\n");
const bits = 0x0102030405060708n;

const frame = defs.encode([{ form: "raw", channel: 1, bits }]).frame;
assert.equal(defs.decode(frame)[0].raw, bits);
```

A value written the way a definition file writes one — `0x` for a bit
pattern, anything else for a physical value — is read by `parseValue`,
which returns `undefined` for text that denotes no value at all. That is
what a field the user is still typing into needs.

```js
assert.deepEqual(parseValue("0x1F", 1), { form: "raw", channel: 1, bits: 31n });
assert.deepEqual(parseValue("12.5", 1), { form: "physical", channel: 1, value: 12.5 });
assert.equal(parseValue("", 1), undefined);
```

## Headers in another language

A column has one canonical name, and every other spelling a header may use
is a vocabulary you supply. The one shipped for the Japanese column names
has no standing one you build lacks.

```js
const german = new ColumnVocabulary();
german.ch("Nummer", "number");
german.ch("Bytes", "bytes");
german.ch("Bezeichnung", "name");

const defs = Definitions.parse("Nummer,Bytes,Bezeichnung\n7,4,Frame\n", null, german);

assert.deepEqual(defs.issues, []);
assert.equal(defs.channels[0].number, 7);
assert.equal(defs.channels[0].name, "Frame");
```

## Editing a definition file

`Table` is the file as its cells — comment rows, blank rows and unknown
columns included — and it writes back in the shape it was read, byte for
byte when the file already follows the write rules.

```js
const table = Table.parse("number,bytes,memo\r\n1,2,first\r\n");

assert.deepEqual(table.header, ["number", "bytes", "memo"]);
assert.equal(table.cell(0, 2), "first");

table.setCell(0, 1, "4");
assert.equal(table.toCsv(), "number,bytes,memo\r\n1,4,first\r\n");
```

An editor wants to mark the cell, not print a sentence about it, so a
finding carries the row and the column it is about.

```js
const table = Table.parse(
  "number,bytes,type,lsb,min,max,default\n1,2,UI,1,0,100,150\n");
const [finding] = table.defaultsOutOfRange();

assert.equal(finding.code, "value_out_of_range");
assert.equal(table.header[finding.col], "default");
assert.equal(table.cell(finding.row, finding.col), "150");
```

## Freeing

`Definitions`, `Table` and `ColumnVocabulary` hold WebAssembly memory and
have a `free()`. Everything they return — a channel, a reading, an Issue —
is a plain object you may keep, spread, clone or post to a worker, and
never have to free.

## Where to look next

- [The guide](https://github.com/ShortArrow/chdef/blob/main/docs/guide.md)
  — the shortest path through each task
- [The specification](https://github.com/ShortArrow/chdef/blob/main/docs/spec/README.md)
  — what the format is, exactly
- [Migration](https://github.com/ShortArrow/chdef/blob/main/docs/migration.md)
  — what changed between 0.0.x releases

## License

MIT OR Apache-2.0.
