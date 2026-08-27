# ADR-0033: Every binding carries the same surface, and a test holds them to it

- Status: Accepted
- Date: 2026-08-28
- Release: 0.0.16

## Context

chdef has two consumer surfaces: the C ABI of `crates/chdef-capi`, with
the .NET binding built over it, and the WebAssembly binding of
`crates/chdef-wasm` published to npm.
[ADR-0023](./0023-the-abi-carries-every-rule.md) says the ABI carries
every rule a consumer would otherwise reimplement. It says that of each
binding on its own. Nothing compared the bindings with each other.

The golden vectors do not close that gap. They certify arithmetic: given
these definitions and these values, these bytes. A binding that never
declares a call is never asked one of those questions, and every vector
still passes.

Comparing the two surfaces found three divergences. The JavaScript
binding had `fitsWidth`, `rangeOf`, `Table.issues`,
`Table.defaultsOutOfRange` and a vocabulary argument on `Table.parse`;
the C ABI and .NET had none of them. No binding converted a single value
between raw and physical, so a cell editor showing one number
reimplemented `raw × lsb + offset` and the clamp of
[conversion.md](../spec/conversion.md) §2 — the rule ADR-0023 exists to
keep in one place. And the C grid handle held an uninterpreted `Grid`,
which takes the first record as the header unconditionally, while the
crate and JavaScript take it as the header only when it names `number`.
For a file without a header, the row number in a finding pointed at a
different cell through C than through JavaScript.

## Decision

**`docs/spec/abi.md` §3 carries a table, one row per operation, naming
it in C, .NET and JavaScript.** The table is the contract. A name in the
table is a name that binding declares.

**`crates/chdef-capi/tests/surface_parity.rs` reads the table** and
fails when the C header, the .NET sources or the WebAssembly source lack
a name it gives them, and when `abi.jp.md` lists different names. The
.NET and JavaScript sources are read as text: this crate's tests run
without a .NET or Node toolchain, and a name that is absent from the
source is absent from the compiled binding too.

**The C grid handle holds a `ChTable` read with a vocabulary**
(`chdef_grid_parse_with`), so the header rule and the rowed findings
(`chdef_grid_issues`, `chdef_grid_defaults_out_of_range`) are the
crate's rather than the C layer's. A binding's grid is still exposed as
cells ([ADR-0020](./0020-a-grid-is-the-uninterpreted-file.md)); the
vocabulary decides which record is the header and what a finding points
at.

The calls each binding lacked were added to it. `CHDEF_ABI_VERSION` is 7.

What the table does not unify:

- **Names follow each language's convention** — `chdef_grid_to_csv`,
  `Grid.ToCsv`, `Table.toCsv`. A single spelling would be wrong in two
  of the three.
- **C addresses a channel by index, JavaScript by channel number.** The
  index is what a C array walk has; the number is what a JavaScript
  caller holds.
- **`ColumnVocabulary::with` is not exposed.** Teaching a spelling onto
  `Japanese()` composes already, and a symbol once exported is never
  withdrawn.

## Consequences

Adding a call to one binding and not the others fails the build. The
divergence is caught by a test rather than by a reviewer noticing.

Changing `chdef_grid_parse`'s header rule is a behaviour change for a C
or .NET caller whose files have no header: they gain one data row, and
the row numbers in their findings move with it. JavaScript callers are
unaffected — they had the crate's rule already.

The table is prose, maintained in two languages. The test holds the two
pages in step.
