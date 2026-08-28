//! A definition with any Issue is refused (`docs/spec/embedded.md` §3).
//! A row the host would load with a warning does not reach a device, where
//! nothing can warn — and the findings come back as `chdef` reports them,
//! pointing at the row that has to be fixed.

use chdef::ColumnVocabulary;
use chdef_core::Endian;
use chdef_gen::{model, Refusal};

const UNKNOWN_TYPE: &[u8] = b"number,bytes,type\n1,2,XX\n";

#[test]
fn a_finding_the_host_would_only_warn_about_refuses_the_table() {
    let Err(refusal) = model(UNKNOWN_TYPE, b"", Endian::Little, &ColumnVocabulary::new()) else {
        panic!("a definition with a finding became a table");
    };
    let Refusal::Issues(issues) = &refusal else {
        panic!("expected the findings, got {refusal:?}");
    };

    assert!(!issues.is_empty());
    assert!(
        refusal.to_string().contains(issues[0].code.as_str()),
        "the refusal does not name the code: {refusal}"
    );
}

#[test]
fn a_finding_tied_to_a_row_says_which() {
    let Err(refusal) = model(UNKNOWN_TYPE, b"", Endian::Little, &ColumnVocabulary::new()) else {
        panic!("a definition with a finding became a table");
    };

    assert!(
        refusal.to_string().contains("row"),
        "the refusal does not point at the row: {refusal}"
    );
}
