// What the JavaScript binding promises beyond the arithmetic the golden
// vectors already certify: the shape the values arrive in, and what
// happens to input JavaScript is free to spell any way it likes.

import { test } from "node:test";
import assert from "node:assert/strict";

import chdef from "@shortarrow/chdef";

const CH = "number,bytes,type,lsb,offset,unit,min,max,default,name\n"
         + "1,2,UI,0.5,0,km/h,0,100,20,speed\n"
         + "2,1,UI,1,0,,,,1,status\n";
const BF = "channel,bit,name,default\n2,0,ready,1\n2,1,fault,0\n";

const defs = () => chdef.Definitions.parse(CH, BF);

test("a record is a plain object, so a caller may keep it", () => {
  const channel = defs().channels[0];

  assert.equal(Object.getPrototypeOf(channel), Object.prototype);
  assert.equal(channel.free, undefined);
  assert.equal({ ...channel }.name, "speed");
});

test("a bit pattern is a bigint, which structuredClone carries and JSON does not", () => {
  const channel = defs().channels[0];

  assert.equal(typeof channel.default, "bigint");
  assert.equal(structuredClone(channel).default, 20n);
  assert.throws(() => JSON.stringify(channel), TypeError);
});

test("a record survives structuredClone, so it may be posted to a worker", () => {
  const reading = defs().decode(Uint8Array.from([0x40, 0x00, 0x01]))[0];
  const copy = structuredClone(reading);

  assert.deepEqual(copy, reading);
  assert.equal(copy.value, 32);
});

test("the three things that own memory are handles, and say so", () => {
  for (const handle of [
    defs(),
    chdef.Table.parse(CH),
    chdef.ColumnVocabulary.japanese(),
  ]) {
    assert.equal(typeof handle.free, "function");
    handle.free();
  }
});

test("a raw value wider than a JavaScript number survives the round trip", () => {
  const wide = chdef.Definitions.parse("number,bytes,type,name\n1,8,UI,counter\n");
  const bits = 0x0102030405060708n;

  const frame = wide.encode([{ form: "raw", channel: 1, bits }]).frame;
  assert.equal(wide.decode(frame)[0].raw, bits);
});

test("a value the caller spelled wrong is thrown back, not read as zero", () => {
  assert.throws(() => defs().encode([{ form: "physical", channel: 1 }]));
  assert.throws(() => defs().encode([{ channel: 1, value: 3 }]));
  assert.throws(() => {
    defs().endian = "middle";
  });
});

test("text that denotes no value is undefined, not an exception", () => {
  assert.equal(chdef.parseValue("", 1), undefined);
  assert.deepEqual(chdef.parseValue("0x1F", 1), { form: "raw", channel: 1, bits: 31n });
  assert.deepEqual(chdef.parseValue("12.5", 1), { form: "physical", channel: 1, value: 12.5 });
});

test("a definition file reads and writes back as the cells it was read as", () => {
  const table = chdef.Table.parse(CH);
  const before = table.toCsv();

  table.appendRow();
  assert.equal(table.rowCount, 3);
  assert.equal(table.removeRow(2), true);
  assert.equal(table.removeRow(9), false);
  assert.equal(table.toCsv(), before);
});

test("a default outside its own row's range names the cell to colour", () => {
  const table = chdef.Table.parse(
    "number,bytes,type,lsb,min,max,default\n1,2,UI,1,0,100,150\n");
  const [finding] = table.defaultsOutOfRange();

  assert.equal(finding.row, 0);
  assert.equal(table.header[finding.col], "default");
  assert.equal(table.cell(finding.row, finding.col), "150");

  table.setCell(0, finding.col, "80");
  assert.deepEqual(table.defaultsOutOfRange(), []);
});

test("a sealed frame satisfies the recipe an unsealed one does not", () => {
  const withCrc = chdef.Definitions.parse(
    "number,bytes,type,kind,derived,name\n"
    + "1,2,UI,plain,,speed\n"
    + "2,2,UI,derived,crc16/x25 1..1,crc\n");

  const open = withCrc.encode([{ form: "physical", channel: 1, value: 7 }]).frame;
  assert.equal(withCrc.derivedMismatches(open).length, 1);

  const sealed = withCrc.seal(open).frame;
  assert.deepEqual(withCrc.derivedMismatches(sealed), []);
  assert.deepEqual([...withCrc.coveredBytes(2, sealed)], [...open.slice(0, 2)]);
});

test("a vocabulary is data, and Japanese is one that ships", () => {
  const japanese = chdef.ColumnVocabulary.japanese();
  const read = chdef.Definitions.parse(
    "番号,バイト数,型,信号名称\n1,2,UI,速度\n", null, japanese);

  assert.deepEqual(read.issues, []);
  assert.equal(read.channels[0].name, "速度");
});

test("one value converts either way, by the rules a whole frame uses", () => {
  const d = defs();
  // conversion.md §2: half away from zero, clamped to the width.
  assert.equal(d.toRaw(1, 0.25), 1n);
  assert.equal(d.toRaw(1, 1e9), 0xFFFFn);
  assert.equal(d.toRaw(1, NaN), undefined);
  assert.equal(d.toRaw(9, 1), undefined);
  // conversion.md §1: raw × lsb + offset, bits beyond the width ignored.
  assert.equal(d.toValue(1, 64n), 32);
  assert.equal(d.toValue(1, 0x1_0040n), 32);
  assert.equal(d.toValue(9, 1n), undefined);

  const { frame } = d.encode([{ form: "physical", channel: 1, value: 32 }]);
  assert.equal(d.toRaw(1, 32), BigInt(frame[0] | (frame[1] << 8)));
});

test("whether a width holds a value, and what range a channel declares", () => {
  const d = defs();
  assert.equal(d.fitsWidth(1, 65535 * 0.5), true);
  assert.equal(d.fitsWidth(1, 65536 * 0.5), false);
  assert.equal(d.fitsWidth(9, 0), false);
  assert.deepEqual(d.rangeOf(1), { min: 0, max: 100 });
  assert.deepEqual(d.rangeOf(2), { min: undefined, max: undefined });
  assert.equal(d.rangeOf(9), undefined);
});

test("the canonical column names are listed, so a picker can be built from them", () => {
  const ch = chdef.ColumnVocabulary.chColumns();
  assert.equal(ch[0], "number");
  assert.ok(ch.includes("default"));
  assert.deepEqual(chdef.ColumnVocabulary.bfColumns().slice(0, 2), ["number", "bit"]);

  const words = new chdef.ColumnVocabulary();
  for (const column of ch) assert.equal(words.ch(column.toUpperCase(), column), true);
  assert.equal(words.ch("x", "no such column"), false);
});

test("every Issue a build can report is listed, so a table can be proved complete", () => {
  const codes = chdef.issueCodes();

  assert.ok(codes.length > 0);
  assert.equal(new Set(codes).size, codes.length);
  for (const issue of chdef.Definitions.parse("number,bytes,type\nx,2,UI\n").issues) {
    assert.ok(codes.includes(issue.code), `${issue.code} is not listed`);
  }
});
