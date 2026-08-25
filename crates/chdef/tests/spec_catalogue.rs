//! The recipes and the Issue codes chdef ships are exactly the ones
//! `docs/spec/` prints.
//!
//! Both lists live in two places a person keeps by hand — the
//! specification and the source — and every parallel list in this
//! repository that was not checked has drifted at least once. The
//! discipline is `spec_japanese_vocabulary.rs`: read both artefacts and
//! compare, with the specification as the side that is read *from*.

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

/// The rows of the first Markdown table after `heading`, as their cells.
fn table(markdown: &str, heading: &str) -> Vec<Vec<String>> {
    let start = markdown
        .find(heading)
        .unwrap_or_else(|| panic!("the specification has no {heading:?}"));
    let mut rows = Vec::new();
    let mut seen_any = false;
    for line in markdown[start..].lines().skip(1) {
        let line = line.trim();
        if !line.starts_with('|') {
            if seen_any {
                break;
            }
            continue;
        }
        seen_any = true;
        if line.contains("---") {
            continue;
        }
        rows.push(
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_string())
                .collect(),
        );
    }
    assert!(!rows.is_empty(), "no table after {heading:?}");
    rows
}

fn number(text: &str) -> u64 {
    match text.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).unwrap_or_else(|e| panic!("{text}: {e}")),
        None => text.parse().unwrap_or_else(|e| panic!("{text}: {e}")),
    }
}

// -------------------------------------------------------- the recipes

#[test]
fn the_recipes_shipped_are_the_ones_the_specification_prints() {
    let spec = read("docs/spec/format.md");
    let rows: Vec<Vec<String>> = table(&spec, "| name | width | poly |")
        .into_iter()
        .filter(|row| row[0] != "name")
        .collect();

    let shipped = DerivedRecipe::all();
    assert_eq!(
        rows.iter().map(|r| r[0].as_str()).collect::<Vec<_>>(),
        shipped,
        "the printed names and the shipped names, in order"
    );

    for row in &rows {
        let (name, printed) = (row[0].as_str(), &row[1..]);
        let crc = DerivedRecipe::named(name).unwrap_or_else(|| panic!("{name} is not shipped"));

        assert_eq!(crc.width as u64, number(&printed[0]), "{name} width");
        assert_eq!(crc.poly, number(&printed[1]), "{name} poly");
        assert_eq!(crc.init, number(&printed[2]), "{name} init");
        assert_eq!(crc.refin, printed[3] == "yes", "{name} refin");
        assert_eq!(crc.refout, printed[4] == "yes", "{name} refout");
        assert_eq!(crc.xorout, number(&printed[5]), "{name} xorout");

        // The check value is the one thing here that is not chdef's to
        // decide: it comes from the published CRC catalogue, and it is how
        // a mistyped parameter is caught rather than agreed with.
        assert_eq!(crc.check(), number(&printed[6]), "{name} check value");
    }
}

// ----------------------------------------------------- the issue codes

#[test]
fn the_codes_reported_are_the_ones_the_specification_prints() {
    let spec = read("docs/spec/diagnostics.md");
    let printed: Vec<String> = spec
        .lines()
        .filter(|line| line.trim_start().starts_with("| `"))
        .filter_map(|line| line.trim().trim_matches('|').split('|').next())
        .map(|cell| cell.trim().trim_matches('`').to_string())
        .filter(|code| code.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .collect();
    assert!(printed.len() >= 30, "found only {}", printed.len());

    let reported: Vec<&str> = IssueCode::all().iter().map(|c| c.as_str()).collect();

    let undocumented: Vec<&&str> = reported
        .iter()
        .filter(|c| !printed.contains(&c.to_string()))
        .collect();
    assert!(
        undocumented.is_empty(),
        "reported but never printed by the specification: {undocumented:?}"
    );

    let unreported: Vec<&String> = printed
        .iter()
        .filter(|c| !reported.contains(&c.as_str()))
        .collect();
    assert!(
        unreported.is_empty(),
        "printed by the specification but never reported: {unreported:?}"
    );
}
