//! Reading a CH or BF CSV as nothing but its cells
//! (`docs/spec/layout.md` §1, the Table stage). A consumer that does not
//! interpret the columns should not have to pick between two names for the
//! same grid.

use chdef::*;

const CH: &str = "number,bytes,name,謎の列\n1,4,Frame,keep\n# note\n2,2,Status,\n";
const BF: &str = "number,bit,name\n2,0,alive\n2,1,ready\n";

// The same call reads either file: which columns they name is not the
// grid's business.
#[test]
fn one_entry_point_reads_a_ch_file_and_a_bf_file() {
    let ch = Grid::parse(CH).unwrap();
    let bf = Grid::parse(BF).unwrap();

    assert_eq!(ch.header().map(|h| h.len()), Some(4));
    assert_eq!(ch.row_count(), 3);
    assert_eq!(bf.header().map(|h| h.len()), Some(3));
    assert_eq!(bf.row_count(), 2);
}

// Cells come back verbatim, unknown columns and comment rows included.
#[test]
fn the_cells_come_back_verbatim() {
    let grid = Grid::parse(CH).unwrap();

    assert_eq!(grid.header().map(|h| h[3].as_str()), Some("謎の列"));
    assert_eq!(grid.cell(0, 3), Some("keep"));
    assert_eq!(grid.row(1).map(|r| r[0].as_str()), Some("# note"));
    assert_eq!(grid.rows().count(), 3);
    assert_eq!(grid.cell(9, 0), None);
}

// The grid is editable, and writes back in the shape it was read.
#[test]
fn the_grid_is_editable_and_keeps_the_file_shape() {
    let mut grid = Grid::parse(CH).unwrap();

    grid.set_cell(0, 2, "Renamed");
    grid.append_row(vec!["3".into(), "1".into(), "Mode".into(), String::new()]);
    assert_eq!(
        grid.remove_row(1).map(|r| r[0].clone()),
        Some("# note".into())
    );

    let written = grid.to_csv();
    assert!(written.contains("1,4,Renamed,keep\n"));
    assert!(written.contains("3,1,Mode,\n"));
    assert!(!written.contains("# note"));
    assert_eq!(grid.style().line_ending, LineEnding::Lf);
    assert!(!written.starts_with('\u{FEFF}'));
}

// Bytes go through the same door, byte-order mark and all.
#[test]
fn bytes_go_through_the_same_door() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"number,name\r\n1,a\r\n");

    let grid = Grid::parse_bytes(&bytes).unwrap();

    assert_eq!(grid.header().map(|h| h.len()), Some(2));
    assert_eq!(grid.style(), CsvStyle::default());
    assert_eq!(grid.to_csv(), "\u{FEFF}number,name\r\n1,a\r\n");
}

// A structurally broken file is refused here too
// (`docs/spec/diagnostics.md` §1).
#[test]
fn a_structurally_broken_file_is_refused() {
    let broken = Grid::parse("number,name\n1,\"never closed\n2,b\n");

    assert!(matches!(broken, Err(ChdefError::CsvParse { .. })));
}

// A typed table is a grid plus a column vocabulary, so it can hand the
// grid over to code that wants only the cells.
#[test]
fn a_typed_table_hands_over_its_grid() {
    let table = ChTable::parse(CH).unwrap();

    let grid = table.grid();

    assert_eq!(grid.header(), table.header());
    assert_eq!(grid.row_count(), table.row_count());
    assert_eq!(grid.cell(0, 2), Some("Frame"));
}

// An empty file is an empty grid, not an error.
#[test]
fn an_empty_file_is_an_empty_grid() {
    let grid = Grid::parse("").unwrap();

    assert_eq!(grid.header(), None);
    assert_eq!(grid.row_count(), 0);
    assert_eq!(grid.to_csv(), "");
}

// A grid created in code writes the defaults of `format.md` §1.
#[test]
fn a_grid_created_in_code_writes_the_defaults() {
    let mut grid = Grid::new();
    grid.append_row(vec!["1".into(), "a".into()]);

    assert_eq!(grid.to_csv(), "\u{FEFF}1,a\r\n");
    assert_eq!(grid.header(), None);
}

// The Table JSON of `docs/spec/interchange.md` §2 is the grid's, not a
// typed table's.
#[cfg(feature = "serde")]
#[test]
fn the_table_json_is_the_grids() {
    let grid = Grid::parse(CH).unwrap();

    let json = serde_json::to_value(grid.to_json()).unwrap();

    assert_eq!(json["header"][3], "謎の列");
    assert_eq!(json["rows"][1][0], "# note");
}
