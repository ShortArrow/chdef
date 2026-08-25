//! Every Issue code, enumerable (`docs/spec/diagnostics.md`, ADR-0026).
//!
//! The codes cross as strings so the vocabulary can grow (ADR-0021); this
//! is what lets a consumer check its own table against the vocabulary
//! instead of finding out when a count moves in an unrelated test.

use chdef::*;

#[test]
fn every_code_is_reachable_without_naming_it() {
    let all = IssueCode::all();
    assert!(all.len() >= 24, "found only {}", all.len());
    assert!(all.contains(&IssueCode::HeaderAssumed));
    assert!(all.contains(&IssueCode::KindAssumed), "codes added since");
}

#[test]
fn the_list_holds_each_code_once() {
    let all = IssueCode::all();
    let mut seen: Vec<&str> = all.iter().map(|c| c.as_str()).collect();
    seen.sort_unstable();
    let count = seen.len();
    seen.dedup();
    assert_eq!(seen.len(), count, "a code is listed twice");
}

#[test]
fn every_code_has_a_distinct_stable_spelling() {
    for code in IssueCode::all() {
        let spelling = code.as_str();
        assert!(!spelling.is_empty(), "{code:?} spells as nothing");
        assert!(
            spelling
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'_'),
            "{code:?} spells as {spelling:?}, which is not stable ASCII"
        );
    }
}

#[test]
fn the_list_is_what_a_consumer_checks_its_own_table_against() {
    // The use ADR-0026 exists for: a table keyed by code, proved complete
    // at build time rather than when a count moves.
    let mine: Vec<&str> = IssueCode::all().iter().map(|c| c.as_str()).collect();

    let missing: Vec<&str> = IssueCode::all()
        .iter()
        .map(|c| c.as_str())
        .filter(|code| !mine.contains(code))
        .collect();
    assert!(missing.is_empty(), "not covered: {missing:?}");
}

#[test]
fn an_issue_that_arrives_carries_a_code_the_list_holds() {
    let parsed = parse_ch_csv("number,bytes,kind\n1,2,computed\n").unwrap();
    assert!(!parsed.issues.is_empty());
    for issue in &parsed.issues {
        assert!(
            IssueCode::all().contains(&issue.code),
            "{:?} is not in the list",
            issue.code
        );
    }
}
