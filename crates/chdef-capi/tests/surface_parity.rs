//! Every binding carries the same surface, and `docs/spec/abi.md` §3 says
//! under which name. A binding can grow a call the others lack without
//! anything failing — the golden vectors certify arithmetic, not
//! coverage — so this test holds each binding to the table: a name the
//! table gives a binding must be declared there, and the Japanese page
//! must list the same names.
//!
//! The .NET and JavaScript sources are read as text rather than compiled,
//! since this crate's tests run without either toolchain.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// One operation and its three names, as the table lists them.
struct Row {
    operation: String,
    c: String,
    dotnet: String,
    js: String,
}

/// The rows of the first table under "The same surface, by name".
fn surface_table(page: &str, heading: &str) -> Vec<Row> {
    let section = page
        .split(heading)
        .nth(1)
        .unwrap_or_else(|| panic!("no heading {heading:?}"));
    section
        .lines()
        .filter(|line| line.starts_with("| ") && line.matches('`').count() == 6)
        .map(|line| {
            let cells: Vec<&str> = line
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().trim_matches('`'))
                .collect();
            assert_eq!(cells.len(), 4, "a row with other than four cells: {line}");
            Row {
                operation: cells[0].to_string(),
                c: cells[1].to_string(),
                dotnet: cells[2].to_string(),
                js: cells[3].to_string(),
            }
        })
        .collect()
}

fn english() -> Vec<Row> {
    surface_table(&read("docs/spec/abi.md"), "### The same surface, by name")
}

/// The last identifier of a dotted or constructed name: `Grid.Parse` is
/// `Parse`, `new ColumnVocabulary()` is `ColumnVocabulary`.
fn member(name: &str) -> &str {
    name.trim_end_matches("()")
        .rsplit(['.', ' '])
        .next()
        .unwrap()
}

/// Whether `word` appears in `text` as a whole identifier.
fn declares(text: &str, word: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

#[test]
fn the_table_names_every_operation_once_per_binding() {
    let rows = english();
    assert!(rows.len() >= 40, "found only {} rows", rows.len());
    for row in &rows {
        for (binding, name) in [
            ("C", &row.c),
            (".NET", &row.dotnet),
            ("JavaScript", &row.js),
        ] {
            assert!(
                !name.is_empty(),
                "{:?} has no {binding} name in the table",
                row.operation
            );
        }
    }
}

#[test]
fn the_header_declares_every_c_name_in_the_table() {
    let header = read("crates/chdef-capi/include/chdef.h");
    for row in english() {
        assert!(
            header.contains(&format!("{}(", row.c)),
            "{:?}: include/chdef.h declares no `{}`",
            row.operation,
            row.c
        );
    }
}

#[test]
fn the_dotnet_binding_declares_every_dotnet_name_in_the_table() {
    let sources = [
        "bindings/dotnet/Chdef/Definitions.cs",
        "bindings/dotnet/Chdef/Grid.cs",
        "bindings/dotnet/Chdef/Vocabulary.cs",
        "bindings/dotnet/Chdef/IssueCode.cs",
    ]
    .iter()
    .map(|p| read(p))
    .collect::<Vec<_>>()
    .join("\n");
    for row in english() {
        let name = member(&row.dotnet);
        assert!(
            declares(&sources, name),
            "{:?}: no public member `{name}` in bindings/dotnet/Chdef",
            row.operation
        );
    }
}

#[test]
fn the_javascript_binding_declares_every_javascript_name_in_the_table() {
    let source = read("crates/chdef-wasm/src/lib.rs");
    for row in english() {
        let name = member(&row.js);
        let exported = source.contains(&format!("js_name = \"{name}\""))
            || source.contains(&format!("js_class = \"{name}\""))
            || source.contains(&format!("pub fn {name}("))
            || source.contains(&format!("pub fn set_{name}("))
            || source.contains(&format!("pub fn {name}<"))
            || source.contains(&format!("pub struct {name} "))
            || source.contains(&format!("pub {name}:"));
        assert!(
            exported,
            "{:?}: crates/chdef-wasm exports nothing named `{name}`",
            row.operation
        );
    }
}

#[test]
fn the_japanese_page_lists_the_same_names() {
    let jp = surface_table(&read("docs/spec/abi.jp.md"), "### 同じ面を、名前で");
    let en = english();
    let names = |rows: &[Row]| -> Vec<(String, String, String)> {
        rows.iter()
            .map(|r| (r.c.clone(), r.dotnet.clone(), r.js.clone()))
            .collect()
    };
    assert_eq!(
        names(&en),
        names(&jp),
        "abi.md and abi.jp.md list different names"
    );
}
