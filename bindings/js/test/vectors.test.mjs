// The golden vectors of docs/spec/interchange.md §3, run through the
// JavaScript binding.
//
// This is what makes shipping a binding worth the cost (ADR-0022): the
// vectors certify the path a JavaScript project actually takes, not a
// path beside it. The same files certify the crate, the C ABI and the
// .NET binding.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import chdef from "chdef";

// The vectors live with the crate that defines what correct is, so the
// binding is measured against the same files and cannot drift into
// agreeing with itself.
const root = join(
  dirname(fileURLToPath(import.meta.url)),
  "..", "..", "..", "crates", "chdef", "vectors",
);

const hex = (bytes) =>
  [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");

const bytes = (text) =>
  Uint8Array.from(text.match(/../g) ?? [], (pair) => parseInt(pair, 16));

const declared = (field) => (field === "-" ? [] : field.split(";"));

function parseValues(field) {
  if (field === "-") return [];
  return field.split(";").map((pair) => {
    const [number, text] = pair.split("=");
    const value = chdef.parseValue(text, Number(number));
    assert.ok(value !== undefined, `unreadable value ${pair}`);
    return value;
  });
}

function checkEncode(defs, values, expectedHex, expectedIssues, at) {
  const encoded = defs.encode(parseValues(values));
  assert.equal(hex(encoded.frame), expectedHex, at);
  assert.deepEqual(
    encoded.issues.map((i) => `${i.code}:${i.channel ?? "-"}`),
    declared(expectedIssues ?? "-"),
    at,
  );
}

function checkDecode(defs, frameHex, expected, at) {
  const readings = defs.decode(bytes(frameHex));
  const wanted = expected.split(";");
  assert.equal(readings.length, wanted.length, `${at}: channel count`);

  wanted.forEach((want, i) => {
    const [number, raw, value] = want.split(/[=/]/);
    assert.equal(readings[i].channel, Number(number), `${at}: number`);
    assert.equal(readings[i].raw, BigInt(raw), `${at}: channel ${number} raw`);
    const expect = Number(value);
    assert.ok(
      Math.abs(readings[i].value - expect) <= 1e-9 * Math.max(Math.abs(expect), 1),
      `${at}: channel ${number} value ${readings[i].value} != ${expect}`,
    );
  });
}

function checkBits(defs, frameHex, expected, at) {
  const readings = defs.decode(bytes(frameHex));
  for (const want of expected.split(";")) {
    const [position, value] = want.split("=");
    const [number, bit] = position.split(":").map(Number);

    const reading = readings.find((r) => r.channel === number);
    assert.ok(reading, `${at}: channel ${number} is not in the frame`);
    const read = reading.bits.find((b) => b.number === bit);
    assert.ok(read, `${at}: bit ${bit} of channel ${number} is not defined`);
    assert.equal(read.value, value === "1", `${at}: bit ${bit} of channel ${number}`);
  }
}

function checkLayout(defs, total, positions, at) {
  assert.equal(String(defs.totalBytes), total, `${at}: totalBytes`);
  assert.deepEqual(
    defs.channels.map((ch) => `${ch.number}:${ch.at}:${ch.bytes}`),
    positions.split(";"),
    `${at}: positions`,
  );
}

for (const set of readdirSync(root, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name)
  .sort()) {
  test(`every golden vector of ${set} holds through the binding`, () => {
    const dir = join(root, set);
    const defs = chdef.Definitions.parse(
      readFileSync(join(dir, "ch.csv"), "utf8"),
      readFileSync(join(dir, "bf.csv"), "utf8"),
    );

    let checkedLines = 0;
    let issueSources = 0;
    const expectedIssues = [];

    readFileSync(join(dir, "vectors.txt"), "utf8").split("\n").forEach((raw, i) => {
      const line = raw.trim();
      if (line === "" || line.startsWith("#")) return;

      const at = `${set}/vectors.txt:${i + 1} (through the JavaScript binding)`;
      const f = line.split(/\s+/);
      switch (f[0]) {
        case "B":
          defs.endian = f[1];
          break;
        case "E":
          checkEncode(defs, f[1], f[2], f[3], at);
          checkedLines++;
          break;
        case "D":
          checkDecode(defs, f[1], f[2], at);
          checkedLines++;
          break;
        case "F":
          checkBits(defs, f[1], f[2], at);
          checkedLines++;
          break;
        case "L":
          checkLayout(defs, f[1], f[2], at);
          checkedLines++;
          break;
        case "P":
          // The `P` lines name Issues per source file; the binding sees
          // the one merged list its parse returns, so what is contracted
          // here is their union.
          expectedIssues.push(...declared(f[2]));
          issueSources++;
          break;
        default:
          assert.fail(`${at}: unreadable vector line ${line}`);
      }
    });

    if (issueSources > 0) {
      assert.deepEqual(
        defs.issues.map((i) => `${i.code}:${i.row ?? "-"}`).sort(),
        expectedIssues.sort(),
      );
    }
    assert.ok(checkedLines > 0, `${set}: nothing was checked through the binding`);
  });
}
