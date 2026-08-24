//! Tests for what a read → edit → write round trip preserves
//! (`docs/spec/editing.md` §2 and the write column of
//! `docs/spec/format.md` §1).

use chdef::*;

// A file that already follows the write rules comes back byte for byte,
// so a consumer that edits one cell gets a one-line diff.
#[test]
fn a_file_that_follows_the_write_rules_round_trips_byte_for_byte() {
    let source = "\u{FEFF}number,bytes,name\r\n1,2,Status\r\n2,4,\"a, comma\"\r\n";

    let table = ChTable::parse(source).unwrap();

    assert_eq!(table.to_csv(), source);
}

// The shape of the file is read, not imposed: a definition kept in a
// repository with LF endings and no byte-order mark stays that way.
#[test]
fn a_file_without_a_bom_and_with_lf_endings_keeps_both() {
    let source = "number,bytes,name\n1,2,Status\n";

    let written = ChTable::parse(source).unwrap().to_csv();

    assert_eq!(written, source);
    assert!(!written.starts_with('\u{FEFF}'));
    assert!(!written.contains('\r'));
}

// And the other shape likewise.
#[test]
fn a_file_with_a_bom_and_crlf_endings_keeps_both() {
    let source = "\u{FEFF}number,name\r\n1,a\r\n";

    let written = ChTable::parse(source).unwrap().to_csv();

    assert_eq!(written, source);
}

// format.md §1 write column: a file chdef creates uses a BOM and `\r\n`,
// so spreadsheet software does not guess another encoding.
#[test]
fn a_new_table_uses_the_write_defaults() {
    let written = ChTable::new().to_csv();

    assert!(written.starts_with('\u{FEFF}'));
    assert!(written.contains("\r\n"));
    assert_eq!(BfTable::new().style(), CsvStyle::default());
}

// The shape is a value the consumer can read and set, so a project that
// wants one shape everywhere can impose it.
#[test]
fn the_shape_can_be_read_and_set() {
    let mut table = ChTable::parse("number,name\n1,a\n").unwrap();

    assert_eq!(
        table.style(),
        CsvStyle {
            bom: false,
            line_ending: LineEnding::Lf,
        }
    );

    table.set_style(CsvStyle::default());

    assert_eq!(table.to_csv(), "\u{FEFF}number,name\r\n1,a\r\n");
}

// format.md §1: a newline inside a quoted cell is part of the cell, so it
// says nothing about how the file separates records.
#[test]
fn a_newline_inside_a_cell_does_not_decide_the_record_separator() {
    let source = "number,memo\r\n1,\"line one\nline two\"\r\n";

    let table = ChTable::parse(source).unwrap();

    assert_eq!(table.style().line_ending, LineEnding::Crlf);
    assert_eq!(table.to_csv(), source);
}

// editing.md §2: quoting is still normalised — a cell is quoted only when
// it needs to be, whatever the source did.
#[test]
fn unnecessary_quotes_are_still_dropped() {
    let table = ChTable::parse("number,name\n1,\"plain\"\n").unwrap();

    assert_eq!(table.to_csv(), "number,name\n1,plain\n");
}

// A cell that needs quoting gets it, in the shape the file uses.
#[test]
fn a_cell_that_needs_quoting_is_quoted_in_the_file_shape() {
    let mut table = ChTable::parse("number,name\n1,a\n").unwrap();
    table.set_cell(0, 1, "say \"hi\"");

    assert_eq!(table.to_csv(), "number,name\n1,\"say \"\"hi\"\"\"\n");
}

// The BF table reads and writes its shape the same way.
#[test]
fn a_bf_table_keeps_its_shape_too() {
    let source = "number,bit,name\n2,0,alive\n";

    assert_eq!(BfTable::parse(source).unwrap().to_csv(), source);
}
