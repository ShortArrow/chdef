//! Tests derived from the wording of `docs/spec/diagnostics.md` §1 / §3 and
//! `docs/spec/format.md` §1, on the line between a per-row Issue and a
//! fatal error.
//!
//! diagnostics.md §1: "Loading stops only when the file cannot be opened
//! (I/O) or the CSV is structurally broken (an unterminated quote). Those
//! are errors."

use chdef::*;

// §1: a structurally broken CSV is an error. It must never come back as a
// shorter definition — format.md §1 ("Why quote") exists because rows going
// missing is the failure this rule prevents.
#[test]
fn an_unterminated_quote_is_an_error() {
    let broken = "number,name\n1,\"unterminated\n2,b\n3,c\n";

    let error = parse_ch_csv(broken).expect_err("an unterminated quote is structural");

    assert!(
        matches!(error, ChdefError::CsvParse { .. }),
        "expected a structural error, got {error:?}"
    );
}

// §3: the error names where the file broke, so an editor can point at it.
#[test]
fn the_error_names_the_line_the_quote_opened_on() {
    let broken = "number,name\n1,ok\n2,\"never closed\n3,c\n";

    match parse_ch_csv(broken).unwrap_err() {
        ChdefError::CsvParse { line, .. } => assert_eq!(line, 3),
        other => panic!("expected a structural error, got {other:?}"),
    }
}

// format.md §1: "`\n` / `\r\n` inside a quoted cell is part of the cell
// (RFC 4180)." A closed quote is not broken, however many lines it spans.
#[test]
fn a_closed_quote_spanning_lines_is_not_an_error() {
    let text = "number,name,memo\n1,a,\"line one\nline two\"\n2,b,\n";

    let parsed = parse_ch_csv(text).expect("a closed quoted cell is legal");

    assert_eq!(parsed.value.len(), 2);
    assert_eq!(parsed.value[0].memo, "line one\nline two");
}

// format.md §1: "A `\"` outside quotes is a literal character." It opens
// nothing, so it cannot leave anything unterminated.
#[test]
fn a_quote_outside_quotes_is_a_literal_character() {
    let text = "number,name\n1,ab\"cd\n2,e\n";

    let parsed = parse_ch_csv(text).expect("a literal quote is legal");

    assert_eq!(parsed.value.len(), 2);
    assert_eq!(parsed.value[0].name, "ab\"cd");
}

// The doubled quote of an inner `"` is not an opening one either.
#[test]
fn an_escaped_inner_quote_does_not_leave_the_cell_open() {
    let text = "number,name\n1,\"say \"\"hi\"\"\"\n2,b\n";

    let parsed = parse_ch_csv(text).expect("an escaped inner quote is legal");

    assert_eq!(parsed.value.len(), 2);
    assert_eq!(parsed.value[0].name, "say \"hi\"");
}

// The same rule holds for every entry point, including the Table stage an
// editor loads through.
#[test]
fn every_entry_point_refuses_a_structurally_broken_file() {
    let broken = "number,name\n1,\"unterminated\n2,b\n";

    assert!(parse_ch_csv(broken).is_err());
    assert!(parse_bf_csv(broken).is_err());
    assert!(parse_ch_csv_bytes(broken.as_bytes()).is_err());
    assert!(ChTable::parse(broken).is_err());
    assert!(BfTable::parse(broken).is_err());
}

// §1: everything else is an Issue, not an error. A file that is merely
// wrong still loads.
#[test]
fn a_file_that_is_only_wrong_still_loads() {
    let wrong = "number,bytes,type\nx,2,UI\n1,99,ZZ\n";

    let parsed = parse_ch_csv(wrong).expect("wrong is not broken");

    assert_eq!(parsed.value.len(), 1);
    assert!(parsed.issues.len() >= 3);
}
