//! The grid across the C ABI, where it is more than cells: which record is
//! the header (`docs/spec/format.md` §2), and what is wrong with the cells
//! as they stand (`docs/spec/diagnostics.md`, `docs/spec/conversion.md`
//! §8). The cells themselves are `abi.rs`.

use std::ffi::c_char;
use std::ptr;

use chdef_capi::*;

struct Grid(*mut ChdefGrid);

impl Grid {
    fn parse(text: &str) -> Grid {
        Grid::parse_with(text, ptr::null())
    }

    fn parse_with(text: &str, vocabulary: *const ChdefVocabulary) -> Grid {
        let mut grid = ptr::null_mut();
        assert_eq!(
            unsafe {
                chdef_grid_parse_with(
                    text.as_ptr(),
                    text.len(),
                    vocabulary,
                    &mut grid,
                    ptr::null_mut(),
                    0,
                )
            },
            CHDEF_OK
        );
        Grid(grid)
    }

    fn header_count(&self) -> u64 {
        unsafe { chdef_grid_header_count(self.0) }
    }

    fn row_count(&self) -> u64 {
        unsafe { chdef_grid_row_count(self.0) }
    }

    fn cell(&self, row: usize, col: usize) -> String {
        text(|buf, cap| unsafe { chdef_grid_cell(self.0, row, col, buf, cap) })
    }

    fn set_cell(&self, row: usize, col: usize, value: &str) {
        assert_eq!(
            unsafe { chdef_grid_set_cell(self.0, row, col, value.as_ptr(), value.len()) },
            CHDEF_OK
        );
    }

    fn issues(&self) -> Vec<(String, i64, i64)> {
        let mut issues = ptr::null_mut();
        assert_eq!(unsafe { chdef_grid_issues(self.0, &mut issues) }, CHDEF_OK);
        described(issues)
    }

    fn defaults_out_of_range(&self) -> Vec<(String, i64, i64)> {
        let mut issues = ptr::null_mut();
        assert_eq!(
            unsafe { chdef_grid_defaults_out_of_range(self.0, &mut issues) },
            CHDEF_OK
        );
        described(issues)
    }
}

impl Drop for Grid {
    fn drop(&mut self) {
        unsafe { chdef_grid_free(self.0) };
    }
}

fn text(mut call: impl FnMut(*mut c_char, usize) -> usize) -> String {
    let needed = call(ptr::null_mut(), 0);
    let mut buf = vec![0u8; needed + 1];
    call(buf.as_mut_ptr() as *mut c_char, buf.len());
    buf.truncate(needed);
    String::from_utf8(buf).unwrap()
}

/// Each finding as `(code, row, col)`, then the list freed.
fn described(issues: *mut ChdefIssues) -> Vec<(String, i64, i64)> {
    let listed = (0..unsafe { chdef_issue_count(issues) } as usize)
        .map(|index| {
            let mut issue = ChdefIssue::default();
            assert_eq!(
                unsafe { chdef_issue_at(issues, index, &mut issue) },
                CHDEF_OK
            );
            let code = text(|buf, cap| unsafe {
                chdef_issue_text(issues, index, CHDEF_ISSUE_CODE, buf, cap)
            });
            (code, issue.row, issue.col)
        })
        .collect();
    unsafe { chdef_issues_free(issues) };
    listed
}

// ------------------------------------------------------------ the header

#[test]
fn the_first_record_is_the_header_only_when_it_names_number() {
    // format.md §2: a header names `number`; a file whose first record
    // does not is read positionally.
    let with = Grid::parse("number,bytes\n1,2\n");
    assert_eq!(with.header_count(), 2);
    assert_eq!(with.row_count(), 1);

    let without = Grid::parse("1,2\n3,4\n");
    assert_eq!(without.header_count(), 0);
    assert_eq!(without.row_count(), 2);
    assert_eq!(without.cell(0, 0), "1", "row 0 is the first record");
}

#[test]
fn a_header_in_another_spelling_is_a_header_with_the_vocabulary_that_knows_it() {
    let mut vocabulary = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_vocabulary_japanese(&mut vocabulary) },
        CHDEF_OK
    );

    let text = "番号,バイト数,型\n1,2,UI\n";
    let known = Grid::parse_with(text, vocabulary);
    assert_eq!(known.header_count(), 3);
    assert_eq!(known.row_count(), 1);

    let unknown = Grid::parse(text);
    assert_eq!(unknown.header_count(), 0);
    assert_eq!(unknown.row_count(), 2);

    unsafe { chdef_vocabulary_free(vocabulary) };
}

#[test]
fn a_handle_of_another_kind_passed_as_the_vocabulary_is_a_handle_error() {
    // abi.md §2: a handle carries a tag, so one kind passed as another is
    // reported rather than read as the wrong type.
    let other = Grid::parse("number\n1\n");
    let mut grid = ptr::null_mut();
    let text = "number\n1\n";
    assert_eq!(
        unsafe {
            chdef_grid_parse_with(
                text.as_ptr(),
                text.len(),
                other.0 as *const ChdefVocabulary,
                &mut grid,
                ptr::null_mut(),
                0,
            )
        },
        CHDEF_ERR_HANDLE
    );
    assert!(grid.is_null());
}

// ---------------------------------------------------------- the findings

#[test]
fn what_is_wrong_with_the_cells_points_at_the_cell() {
    // diagnostics.md: a finding carries the row and column it is about,
    // 0-based with the header excluded.
    let grid = Grid::parse("number,bytes,type\n1,2,UI\n2,2,XX\n");
    let found = grid.issues();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].1, found[0].2), (1, 2));
}

#[test]
fn the_findings_follow_the_cells_as_they_are_edited() {
    let grid = Grid::parse("number,bytes,type\n1,2,XX\n");
    assert_eq!(grid.issues().len(), 1);
    grid.set_cell(0, 2, "UI");
    assert_eq!(grid.issues(), vec![]);
}

#[test]
fn a_default_outside_its_own_rows_range_names_the_default_cell() {
    // conversion.md §8, on the file rather than the layout.
    let grid = Grid::parse("number,bytes,type,lsb,min,max,default\n1,2,UI,1,0,100,150\n");
    assert_eq!(
        grid.defaults_out_of_range(),
        vec![("value_out_of_range".to_string(), 0, 6)]
    );

    grid.set_cell(0, 6, "80");
    assert_eq!(grid.defaults_out_of_range(), vec![]);
}

#[test]
fn a_file_with_no_default_column_has_no_default_findings() {
    let grid = Grid::parse("number,bytes,type,min,max\n1,2,UI,0,100\n");
    assert_eq!(grid.defaults_out_of_range(), vec![]);
}

#[test]
fn a_null_out_pointer_is_a_null_error() {
    let grid = Grid::parse("number\n1\n");
    assert_eq!(
        unsafe { chdef_grid_issues(grid.0, ptr::null_mut()) },
        CHDEF_ERR_NULL
    );
    assert_eq!(
        unsafe { chdef_grid_defaults_out_of_range(grid.0, ptr::null_mut()) },
        CHDEF_ERR_NULL
    );
}
