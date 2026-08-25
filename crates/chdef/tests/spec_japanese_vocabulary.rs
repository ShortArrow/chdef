//! The shipped Japanese vocabulary is exactly the table
//! `docs/spec/format.md` §2 prints.
//!
//! That table exists in two places a person keeps by hand — the
//! specification and `columns.rs` — and the same shape of parallel list has
//! drifted here before. The discipline is the one
//! `chdef-capi/tests/header.rs` uses: read both artefacts and compare,
//! rather than trusting that they agree.
//!
//! Whether the spellings then *read* a file is `spec_vocabulary.rs`.

use std::path::Path;

use chdef::*;

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The `(kind, column, spelling)` triples of one table under a heading, in
/// order. A row is ``| `column` | `a`, `b` |``.
fn printed(kind: &str, after: &str, before: &str) -> Vec<(String, String, String)> {
    let markdown = read("docs/spec/format.md");
    let start = markdown
        .find(after)
        .unwrap_or_else(|| panic!("the specification has no {after:?}"));
    let end = markdown[start..]
        .find(before)
        .map(|i| start + i)
        .unwrap_or(markdown.len());

    let mut triples = Vec::new();
    for line in markdown[start..end].lines() {
        let line = line.trim();
        if !line.starts_with("| `") {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').collect();
        let column = cells[0].trim().trim_matches('`').to_string();
        for spelling in cells[1].split(',') {
            let spelling = spelling.trim().trim_matches('`');
            if !spelling.is_empty() {
                triples.push((kind.to_string(), column.clone(), spelling.to_string()));
            }
        }
    }
    assert!(!triples.is_empty(), "no rows found after {after:?}");
    triples
}

fn specification() -> Vec<(String, String, String)> {
    let mut all = printed("ch", "### The Japanese vocabulary", "For a BF CSV");
    all.extend(printed("bf", "For a BF CSV", "## 3."));
    all
}

/// Every `.ch(…)` / `.bf(…)` the shipped vocabulary teaches, in order.
fn taught() -> Vec<(String, String, String)> {
    let source = read("crates/chdef/src/columns.rs");
    let start = source.find("pub fn japanese()").expect("japanese()");
    let end = source[start..].find("\n    }").map(|i| start + i).unwrap();

    let mut entries = Vec::new();
    for line in source[start..end].lines() {
        let line = line.trim();
        for (call, kind) in [(".ch(\"", "ch"), (".bf(\"", "bf")] {
            if let Some(rest) = line.strip_prefix(call) {
                let (spelling, rest) = rest.split_once('"').expect("a spelling");
                let column = rest
                    .split("Column::")
                    .nth(1)
                    .and_then(|c| c.split(')').next())
                    .expect("a column")
                    .to_lowercase();
                entries.push((kind.to_string(), column, spelling.to_string()));
            }
        }
    }
    entries
}

#[test]
fn the_vocabulary_teaches_exactly_what_the_specification_prints() {
    let (taught, printed) = (taught(), specification());
    assert!(taught.len() >= 31, "found only {} entries", taught.len());

    let extra: Vec<&(String, String, String)> =
        taught.iter().filter(|e| !printed.contains(e)).collect();
    assert!(
        extra.is_empty(),
        "taught by columns.rs but never printed by the specification: {extra:?}"
    );

    let missing: Vec<&(String, String, String)> =
        printed.iter().filter(|e| !taught.contains(e)).collect();
    assert!(
        missing.is_empty(),
        "printed by the specification but never taught: {missing:?}"
    );
}

#[test]
fn each_column_is_written_with_the_first_spelling_of_its_row() {
    // §2: the first spelling taught for a column is the one written, and
    // the table prints them in that order.
    let japanese = ColumnVocabulary::japanese();

    for (kind, column, spelling) in specification() {
        let first = specification()
            .into_iter()
            .find(|(k, c, _)| *k == kind && *c == column)
            .map(|(_, _, s)| s)
            .unwrap();
        if spelling != first {
            continue;
        }
        let written = match kind.as_str() {
            "ch" => japanese
                .ch_spelling(ChColumn::from_header(&column).expect(&column))
                .to_string(),
            _ => japanese
                .bf_spelling(BfColumn::from_header(&column).expect(&column))
                .to_string(),
        };
        assert_eq!(written, first, "{kind} {column}");
    }
}
