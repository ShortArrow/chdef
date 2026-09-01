//! A definition the macro will not expand is a compile error, and the
//! error says what `chdef-gen` would have printed. The only place that can
//! be observed is a compiler run, so each case is a scratch crate this test
//! checks with `cargo check`.

use std::path::PathBuf;
use std::process::{Command, Output};

/// `cargo check` over one scratch crate, built somewhere outside the
/// repository so a failing case leaves nothing behind.
fn check(scratch: &str) -> Output {
    let manifest: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", scratch, "Cargo.toml"]
        .iter()
        .collect();
    let target = std::env::temp_dir()
        .join("chdef-macros-scratch")
        .join(scratch);

    Command::new(env!("CARGO"))
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--message-format")
        .arg("short")
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .expect("cargo runs")
}

#[test]
fn a_definition_with_findings_is_a_compile_error_naming_them() {
    let output = check("refused_crate");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "the crate compiled:\n{stderr}");
    // The P lines of crates/chdef/vectors/diagnostics/vectors.txt.
    for finding in [
        "the definition was refused",
        "channel_number_invalid",
        "bf_bit_invalid",
    ] {
        assert!(stderr.contains(finding), "`{finding}` is not in:\n{stderr}");
    }
}

#[test]
fn a_definition_that_is_not_there_names_the_path_it_looked_for() {
    let output = check("missing_crate");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "the crate compiled:\n{stderr}");
    assert!(
        stderr.contains("no/such.csv"),
        "the path is not in:\n{stderr}"
    );
}
