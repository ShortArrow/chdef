#!/usr/bin/env bash
# The core claims to run without std, an allocator or a float, and the table
# chdef-gen writes claims to compile where it is meant to run. Only a build
# for a bare target says either of those out loud. Run from the repo root.
set -euo pipefail

# `pwd -W` is the Windows spelling Git Bash can give; the paths below
# reach a native cargo, which cannot read an MSYS one out of a manifest.
repo="$(pwd -W 2>/dev/null || pwd)"

echo "== chdef-core builds for thumbv7em-none-eabihf =="
cargo build -p chdef-core --target thumbv7em-none-eabihf --locked

echo "== the C entry points build as a static library =="
cargo rustc -p chdef-core --features c --target thumbv7em-none-eabihf \
  --release --locked --crate-type staticlib -- -C panic=abort
target="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import sys,json;print(json.load(sys.stdin)["target_directory"])')"
staticlib="$target/thumbv7em-none-eabihf/release/libchdef_core.a"
if [ ! -f "$staticlib" ]; then
  echo "expected a static library at $staticlib" >&2
  exit 1
fi

echo "== chdef-gen expands the basic vectors =="
out="$(mktemp -d)"
cargo run -p chdef-gen --locked -- \
  --ch "$repo/crates/chdef/vectors/basic/ch.csv" \
  --bf "$repo/crates/chdef/vectors/basic/bf.csv" \
  --rust "$out/layout.rs" \
  --c "$out/layout.h"

echo "== the generated header compiles as C11 =="
if command -v cc >/dev/null 2>&1; then
  cat >"$out/use.c" <<'USE_C'
#include "layout.h"

int main(void) { return (int)CHDEF_LAYOUT.total; }
USE_C
  cc -std=c11 -Wall -Wextra -Werror -fsyntax-only \
    -I "$out" -I "$repo/crates/chdef-core/include" "$out/use.c"
else
  echo "no C compiler: skipping"
fi

echo "== the generated table compiles against the core on the target =="
mkdir -p "$out/smoke/src"
cat >"$out/smoke/Cargo.toml" <<CARGO_TOML
[package]
name = "smoke"
version = "0.0.0"
edition = "2021"

[dependencies]
chdef-core = { path = "$repo/crates/chdef-core" }

[workspace]
CARGO_TOML
cat >"$out/smoke/src/lib.rs" <<'LIB_RS'
#![no_std]

include!("../../layout.rs");

pub fn total() -> u32 {
    LAYOUT.total
}
LIB_RS
cargo check --manifest-path "$out/smoke/Cargo.toml" --target thumbv7em-none-eabihf

echo "== the macro expands the table on the target =="
# The macro is a host dependency of a crate built for the bare target, and
# it reads its CSV relative to the invoking crate's manifest — which here
# is the scratch directory, not the repo. Copy the definition in beside it.
mkdir -p "$out/macro/src" "$out/macro/def"
cp "$repo/crates/chdef/vectors/basic/ch.csv" \
   "$repo/crates/chdef/vectors/basic/bf.csv" \
   "$out/macro/def/"
cat >"$out/macro/Cargo.toml" <<CARGO_TOML
[package]
name = "macro_smoke"
version = "0.0.0"
edition = "2021"

[dependencies]
chdef-core = { path = "$repo/crates/chdef-core" }
chdef-macros = { path = "$repo/crates/chdef-macros" }

[workspace]
CARGO_TOML
cat >"$out/macro/src/lib.rs" <<'LIB_RS'
#![no_std]

chdef_macros::layout!("def/ch.csv", bf = "def/bf.csv");

pub fn total() -> u32 {
    LAYOUT.total
}
LIB_RS
cargo check --manifest-path "$out/macro/Cargo.toml" --target thumbv7em-none-eabihf

echo "embedded smoke: ok"
