//! The command line: what a firmware build calls, and what it learns when
//! the definition is one no device may be given.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn vectors(set: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .join("chdef/vectors")
        .join(set)
}

/// A directory of this run's own, so two tests never write the same file.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("chdef-gen-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&path).expect("the scratch directory could not be made");
    path
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chdef-gen"))
        .args(arguments)
        .output()
        .expect("chdef-gen could not be run")
}

#[test]
fn a_definition_that_loads_cleanly_is_written_to_both_files() {
    let set = vectors("basic");
    let out = scratch("basic");
    let rust = out.join("layout.rs");
    let c = out.join("layout.h");

    let output = run(&[
        "--ch",
        &set.join("ch.csv").display().to_string(),
        "--bf",
        &set.join("bf.csv").display().to_string(),
        "--rust",
        &rust.display().to_string(),
        "--c",
        &c.display().to_string(),
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for written in [&rust, &c] {
        let contents = std::fs::read_to_string(written).expect("the file was not written");
        assert!(!contents.is_empty(), "{} is empty", written.display());
    }
}

#[test]
fn a_definition_with_findings_is_refused_on_stderr() {
    let set = vectors("diagnostics");
    let out = scratch("diagnostics");

    let output = run(&[
        "--ch",
        &set.join("ch.csv").display().to_string(),
        "--bf",
        &set.join("bf.csv").display().to_string(),
        "--rust",
        &out.join("layout.rs").display().to_string(),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(!output.stderr.is_empty(), "nothing was said about why");
}

#[test]
fn a_call_that_asks_for_nothing_is_answered_with_the_usage() {
    let output = run(&[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("usage:"),
        "no usage line"
    );
}
