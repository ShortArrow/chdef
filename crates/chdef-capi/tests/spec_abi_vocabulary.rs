//! The column vocabulary across the C ABI (`docs/spec/abi.md` §1,
//! ADR-0024): which header spelling denotes which column is a rule a
//! consumer would otherwise reimplement, so it crosses.

use std::ffi::c_char;
use std::ptr;

use chdef_capi::*;

const JA_CH: &str = "番号,バイト数,メッセージ名称,型\n7,4,Frame,UI32\n";
const DE_CH: &str = "Nummer,Bytes,Bezeichnung,Typ\n7,4,Frame,UI32\n";

fn text(mut call: impl FnMut(*mut c_char, usize) -> usize) -> String {
    let needed = call(ptr::null_mut(), 0);
    let mut buf = vec![0u8; needed + 1];
    call(buf.as_mut_ptr() as *mut c_char, buf.len());
    buf.truncate(needed);
    String::from_utf8(buf).unwrap()
}

struct Vocabulary(*mut ChdefVocabulary);

impl Vocabulary {
    fn empty() -> Vocabulary {
        let mut handle = ptr::null_mut();
        assert_eq!(unsafe { chdef_vocabulary_new(&mut handle) }, CHDEF_OK);
        Vocabulary(handle)
    }

    fn japanese() -> Vocabulary {
        let mut handle = ptr::null_mut();
        assert_eq!(unsafe { chdef_vocabulary_japanese(&mut handle) }, CHDEF_OK);
        Vocabulary(handle)
    }

    fn teach(&self, kind: i32, spelling: &str, column: &str) -> i32 {
        unsafe {
            chdef_vocabulary_teach(
                self.0,
                kind,
                spelling.as_ptr(),
                spelling.len(),
                column.as_ptr(),
                column.len(),
            )
        }
    }
}

impl Drop for Vocabulary {
    fn drop(&mut self) {
        unsafe { chdef_vocabulary_free(self.0) };
    }
}

/// Parse a CH definition through the ABI and report the first channel
/// number, or `None` when the header was not read as one.
fn first_number(ch: &str, vocabulary: *const ChdefVocabulary) -> Option<u32> {
    let mut layout = ptr::null_mut();
    let mut issues = ptr::null_mut();
    let status = unsafe {
        chdef_layout_parse_with(
            ch.as_ptr(),
            ch.len(),
            ptr::null(),
            0,
            vocabulary,
            &mut layout,
            &mut issues,
            ptr::null_mut(),
            0,
        )
    };
    assert_eq!(status, CHDEF_OK);

    let assumed = (0..unsafe { chdef_issue_count(issues) } as usize).any(|index| {
        text(|buf, cap| unsafe { chdef_issue_text(issues, index, CHDEF_ISSUE_CODE, buf, cap) })
            == "header_assumed"
    });
    let mut channel = ChdefChannel::default();
    let read = unsafe { chdef_layout_channel_at(layout, 0, &mut channel) } == CHDEF_OK;

    unsafe {
        chdef_issues_free(issues);
        chdef_layout_free(layout);
    }
    (!assumed && read).then_some(channel.number)
}

#[test]
fn the_canonical_column_names_cross_as_strings() {
    // ADR-0021: no enumeration crosses, so adding a column is not an ABI
    // break. ADR-0024: those names are the column identity.
    assert_eq!(chdef_column_count(CHDEF_COLUMNS_CH), 16);
    assert_eq!(chdef_column_count(CHDEF_COLUMNS_BF), 5);
    assert_eq!(chdef_column_count(99), 0);

    let name = |kind, index| text(|buf, cap| unsafe { chdef_column_name(kind, index, buf, cap) });
    assert_eq!(name(CHDEF_COLUMNS_CH, 0), "number");
    assert_eq!(name(CHDEF_COLUMNS_CH, 15), "favorite");
    assert_eq!(name(CHDEF_COLUMNS_BF, 1), "bit");
    assert_eq!(name(CHDEF_COLUMNS_CH, 16), "", "past the end");
}

#[test]
fn a_null_vocabulary_reads_the_canonical_names_alone() {
    assert_eq!(first_number("number,bytes\n7,4\n", ptr::null()), Some(7));
    assert_eq!(
        first_number(JA_CH, ptr::null()),
        None,
        "Japanese is a vocabulary"
    );
}

#[test]
fn the_shipped_vocabulary_reads_the_header_it_names() {
    let japanese = Vocabulary::japanese();
    assert_eq!(first_number(JA_CH, japanese.0), Some(7));
    assert_eq!(
        first_number("number,bytes\n7,4\n", japanese.0),
        Some(7),
        "the canonical names still read"
    );
}

#[test]
fn a_vocabulary_built_through_the_abi_reads_the_same_way() {
    let german = Vocabulary::empty();
    assert_eq!(german.teach(CHDEF_COLUMNS_CH, "Nummer", "number"), CHDEF_OK);
    assert_eq!(german.teach(CHDEF_COLUMNS_CH, "Bytes", "bytes"), CHDEF_OK);
    assert_eq!(first_number(DE_CH, german.0), Some(7));
}

#[test]
fn a_name_no_column_answers_to_is_reported() {
    let vocabulary = Vocabulary::empty();
    assert_eq!(
        vocabulary.teach(CHDEF_COLUMNS_CH, "Nummer", "nonsense"),
        CHDEF_ERR_COLUMN
    );
    assert_eq!(
        vocabulary.teach(CHDEF_COLUMNS_BF, "Bitnummer", "favorite"),
        CHDEF_ERR_COLUMN,
        "a CH column is not a BF column"
    );
    assert_eq!(
        vocabulary.teach(99, "Nummer", "number"),
        CHDEF_ERR_INDEX,
        "an unknown kind"
    );
}

#[test]
fn teaching_survives_a_failed_teaching() {
    // A rejected column name must not cost the caller what it taught
    // before it.
    let vocabulary = Vocabulary::empty();
    assert_eq!(
        vocabulary.teach(CHDEF_COLUMNS_CH, "Nummer", "number"),
        CHDEF_OK
    );
    assert_eq!(
        vocabulary.teach(CHDEF_COLUMNS_CH, "Breite", "nonsense"),
        CHDEF_ERR_COLUMN
    );
    assert_eq!(first_number("Nummer,bytes\n7,4\n", vocabulary.0), Some(7));
}

#[test]
fn a_handle_of_another_kind_is_reported_rather_than_dereferenced() {
    // abi.md §2: a null pointer, or a handle of one kind where another was
    // expected, is CHDEF_ERR_HANDLE. Using a *freed* handle is undefined
    // and is deliberately not asserted on: the memory is the allocator's
    // by then, so any answer would be a coincidence of the platform.
    let grid_source = "number,bytes
7,4
";
    let mut grid = ptr::null_mut();
    assert_eq!(
        unsafe {
            chdef_grid_parse(
                grid_source.as_ptr(),
                grid_source.len(),
                &mut grid,
                ptr::null_mut(),
                0,
            )
        },
        CHDEF_OK
    );

    let borrowed = grid as *mut ChdefVocabulary;
    assert_eq!(
        unsafe {
            chdef_vocabulary_teach(
                borrowed,
                CHDEF_COLUMNS_CH,
                "x".as_ptr(),
                1,
                "number".as_ptr(),
                6,
            )
        },
        CHDEF_ERR_HANDLE,
        "a grid is not a vocabulary"
    );

    let ch = "number,bytes
7,4
";
    let mut layout = ptr::null_mut();
    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe {
            chdef_layout_parse_with(
                ch.as_ptr(),
                ch.len(),
                ptr::null(),
                0,
                borrowed,
                &mut layout,
                &mut issues,
                ptr::null_mut(),
                0,
            )
        },
        CHDEF_ERR_HANDLE
    );

    unsafe { chdef_grid_free(grid) };

    // A null vocabulary is the empty one, not an error — that is the
    // documented shorthand.
    assert_eq!(first_number(ch, ptr::null()), Some(7));

    // A null handle where one is required is an error.
    assert_eq!(
        unsafe {
            chdef_vocabulary_teach(
                ptr::null_mut(),
                CHDEF_COLUMNS_CH,
                "x".as_ptr(),
                1,
                "number".as_ptr(),
                6,
            )
        },
        CHDEF_ERR_HANDLE
    );
    unsafe { chdef_vocabulary_free(ptr::null_mut()) };
}

#[test]
fn the_version_rose_when_the_vocabulary_crossed() {
    assert!(chdef_abi_version() >= 3);
}
