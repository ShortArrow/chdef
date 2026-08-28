//! The header is a checked-in artifact rather than a `cbindgen` build step
//! (ADR-0021), so a test has to prove it cannot fall behind: every entry
//! point and every `repr(C)` type the crate exports must appear in it.
//!
//! This reads the sources as text, so it needs neither the `c` feature nor
//! a C compiler to run.

use std::path::Path;

fn read(relative: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

fn header() -> String {
    read("include/chdef_core.h")
}

/// Every `extern "C"` function name the `c` module exports.
fn exported_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("pub unsafe extern \"C\" fn ")?;
            Some(rest.split('(').next()?.to_string())
        })
        .collect()
}

/// Every `#[repr(C)]` struct a source declares, under the name C knows it
/// by. The Rust types the table is built from are declared without the
/// prefix their C counterparts carry; `Derived` is not `repr(C)` and never
/// crosses on its own, so it has no counterpart to check.
fn exported_types(source: &str) -> Vec<String> {
    let mut types = Vec::new();
    let mut repr_c = false;
    for line in source.lines() {
        let line = line.trim();
        if line == "#[repr(C)]" {
            repr_c = true;
        } else if repr_c {
            if let Some(rest) = line.strip_prefix("pub struct ") {
                let name = rest.trim_end_matches(" {");
                types.push(match name.strip_prefix("ChdefCore") {
                    Some(_) => name.to_string(),
                    None => format!("ChdefCore{name}"),
                });
                repr_c = false;
            } else if !line.starts_with("#[") {
                repr_c = false;
            }
        }
    }
    types
}

#[test]
fn the_header_declares_every_entry_point() {
    let functions = exported_functions(&read("src/c.rs"));
    let header = header();

    assert_eq!(functions.len(), 5, "found {functions:?}");
    for name in functions {
        // The name must appear as a declaration, not merely as a substring
        // of a longer one.
        assert!(
            header.contains(&format!("{name}(")),
            "`{name}` is exported but include/chdef_core.h declares no such function"
        );
    }
}

#[test]
fn the_header_declares_every_exported_type() {
    let mut types = exported_types(&read("src/lib.rs"));
    types.extend(exported_types(&read("src/c.rs")));
    let header = header();

    assert_eq!(
        types,
        [
            "ChdefCoreSlot",
            "ChdefCoreCrc",
            "ChdefCoreRange",
            "ChdefCoreDerived",
            "ChdefCoreLayout",
        ],
        "the crate's repr(C) structs have changed"
    );
    for name in types {
        assert!(
            header.contains(&format!("typedef struct {name} ")),
            "`{name}` is `repr(C)` but include/chdef_core.h declares no such type"
        );
    }
}
