// Builds the two WebAssembly targets this package ships: `bundler` for
// Vite and the other bundlers, `nodejs` for Node. Both come from the same
// Rust crate, so they cannot disagree.

import { execFileSync } from "node:child_process";
import { writeFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const crate = join(here, "..", "..", "crates", "chdef-wasm");

for (const target of ["bundler", "nodejs"]) {
  const out = join(here, target === "nodejs" ? "node" : target);
  rmSync(out, { recursive: true, force: true });
  execFileSync(
    "wasm-pack",
    ["build", crate, "--release", "--target", target,
     "--out-dir", out, "--out-name", "chdef", "--no-pack"],
    { stdio: "inherit" },
  );
  rmSync(join(out, ".gitignore"), { force: true });
}

// The bundler target is ES modules; the Node target is CommonJS, which is
// what this package's own `type` leaves it as.
writeFileSync(
  join(here, "bundler", "package.json"),
  JSON.stringify({ type: "module" }, null, 2) + "\n",
);
