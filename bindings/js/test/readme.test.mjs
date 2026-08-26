// Runs every JavaScript example on the package's front page.
//
// The page is the test rather than a copy of one, so an example cannot
// fall out of step with the binding without the build saying so.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import chdef from "@shortarrow/chdef";

const readme = join(dirname(fileURLToPath(import.meta.url)), "..", "README.md");
const page = readFileSync(readme, "utf8");

const heading = (upto) => {
  const headings = page.slice(0, upto).match(/^## .+$/gm) ?? [];
  return headings.at(-1)?.slice(3) ?? "the opening";
};

const examples = [...page.matchAll(/^```js\n([\s\S]*?)^```$/gm)]
  .map((match) => ({ code: match[1], where: heading(match.index) }));

assert.ok(examples.length > 0, "the front page carries no JavaScript example");

const { Definitions, Table, ColumnVocabulary, parseValue } = chdef;

examples.forEach(({ code, where }, index) => {
  test(`front page example ${index + 1}, under "${where}"`, () => {
    new Function(
      "assert", "Definitions", "Table", "ColumnVocabulary", "parseValue", code,
    )(assert, Definitions, Table, ColumnVocabulary, parseValue);
  });
});
