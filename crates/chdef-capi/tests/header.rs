//! The header is a checked-in artifact rather than a `cbindgen` build step
//! (ADR-0021), so a test has to prove it cannot fall behind: every symbol
//! and every constant the crate exports must appear in it.

use std::path::Path;

fn source() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).unwrap()
}

fn header() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/chdef.h")).unwrap()
}

/// Every `extern "C"` function name in the crate.
fn exported_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line
                .strip_prefix("pub extern \"C\" fn ")
                .or_else(|| line.strip_prefix("pub unsafe extern \"C\" fn "))?;
            Some(rest.split('(').next()?.to_string())
        })
        .collect()
}

/// Every `CHDEF_*` constant the crate declares.
fn exported_constants(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("pub const CHDEF_")?;
            Some(format!("CHDEF_{}", rest.split(':').next()?.trim()))
        })
        .collect()
}

/// Every `#[repr(C)]` type the crate declares.
fn exported_types(source: &str) -> Vec<String> {
    let mut types = Vec::new();
    let mut repr_c = false;
    for line in source.lines() {
        let line = line.trim();
        if line == "#[repr(C)]" {
            repr_c = true;
        } else if repr_c {
            if let Some(rest) = line.strip_prefix("pub struct ") {
                types.push(rest.trim_end_matches(" {").to_string());
                repr_c = false;
            } else if !line.starts_with("#[") {
                repr_c = false;
            }
        }
    }
    types
}

#[test]
fn the_header_declares_every_exported_function() {
    let (source, header) = (source(), header());
    let functions = exported_functions(&source);

    assert!(functions.len() >= 14, "found only {functions:?}");
    for name in functions {
        // The name must appear as a declaration, not merely as a
        // substring of a longer one.
        assert!(
            header.contains(&format!("{name}(")),
            "`{name}` is exported but include/chdef.h declares no such function"
        );
    }
}

#[test]
fn the_header_declares_every_exported_constant() {
    let (source, header) = (source(), header());
    let constants = exported_constants(&source);

    assert!(constants.len() >= 20, "found only {constants:?}");
    for name in constants {
        assert!(
            header.contains(&format!("#define {name} ")),
            "`{name}` is exported but include/chdef.h defines no such constant"
        );
    }
}

#[test]
fn the_header_declares_every_exported_type() {
    let (source, header) = (source(), header());
    let types = exported_types(&source);

    assert!(types.len() >= 4, "found only {types:?}");
    for name in types {
        assert!(
            header.contains(&format!("typedef struct {name} ")),
            "`{name}` is `repr(C)` but include/chdef.h declares no such type"
        );
    }
}

#[test]
fn the_header_states_the_abi_version_the_crate_does() {
    let header = header();

    assert!(
        header.contains(&format!(
            "#define CHDEF_ABI_VERSION {}u",
            chdef_capi::CHDEF_ABI_VERSION
        )),
        "include/chdef.h does not state ABI version {}",
        chdef_capi::CHDEF_ABI_VERSION
    );
}
