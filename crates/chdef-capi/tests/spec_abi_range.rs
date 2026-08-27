//! Asking whether a value is inside its channel's declared range, across
//! the C ABI (`docs/spec/conversion.md` §8).

use std::ffi::c_char;
use std::ptr;

use chdef_capi::*;

const CH: &str = "number,bytes,type,lsb,min,max,default\n\
                  1,2,UI,1,0,100,50\n\
                  2,2,UI,1,,,7\n";

fn text(mut call: impl FnMut(*mut c_char, usize) -> usize) -> String {
    let needed = call(ptr::null_mut(), 0);
    let mut buf = vec![0u8; needed + 1];
    call(buf.as_mut_ptr() as *mut c_char, buf.len());
    buf.truncate(needed);
    String::from_utf8(buf).unwrap()
}

struct Layout(*mut ChdefLayout);

impl Layout {
    fn parse() -> Layout {
        let mut layout = ptr::null_mut();
        let mut issues = ptr::null_mut();
        assert_eq!(
            unsafe {
                chdef_layout_parse(
                    CH.as_ptr(),
                    CH.len(),
                    ptr::null(),
                    0,
                    &mut layout,
                    &mut issues,
                    ptr::null_mut(),
                    0,
                )
            },
            CHDEF_OK
        );
        unsafe { chdef_issues_free(issues) };
        Layout(layout)
    }
}

impl Drop for Layout {
    fn drop(&mut self) {
        unsafe { chdef_layout_free(self.0) };
    }
}

/// The findings of one ask, as `code|found|used|channel`.
fn described(issues: *mut ChdefIssues) -> Vec<String> {
    let listed = (0..unsafe { chdef_issue_count(issues) } as usize)
        .map(|index| {
            let mut issue = ChdefIssue::default();
            assert_eq!(
                unsafe { chdef_issue_at(issues, index, &mut issue) },
                CHDEF_OK
            );
            let field = |which| {
                text(|buf, cap| unsafe { chdef_issue_text(issues, index, which, buf, cap) })
            };
            format!(
                "{}|{}|{}|{}",
                field(CHDEF_ISSUE_CODE),
                field(CHDEF_ISSUE_FOUND),
                field(CHDEF_ISSUE_USED),
                issue.channel
            )
        })
        .collect();
    unsafe { chdef_issues_free(issues) };
    listed
}

fn values_out_of_range(layout: &Layout, values: &[ChdefValue]) -> Vec<String> {
    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_values_out_of_range(layout.0, values.as_ptr(), values.len(), &mut issues) },
        CHDEF_OK
    );
    described(issues)
}

#[test]
fn a_value_outside_its_range_crosses_with_the_bound_it_crossed() {
    let layout = Layout::parse();
    assert_eq!(
        values_out_of_range(&layout, &[ChdefValue::physical(1, 150.0)]),
        vec!["value_out_of_range|150|100|1".to_string()]
    );
}

#[test]
fn a_value_inside_its_range_crosses_as_nothing() {
    let layout = Layout::parse();
    assert!(values_out_of_range(
        &layout,
        &[
            ChdefValue::physical(1, 0.0),
            ChdefValue::physical(1, 100.0),
            ChdefValue::physical(2, 1e9),
        ]
    )
    .is_empty());
}

#[test]
fn a_raw_value_is_judged_by_what_it_means() {
    let layout = Layout::parse();
    assert_eq!(
        values_out_of_range(&layout, &[ChdefValue::raw(1, 150)]),
        vec!["value_out_of_range|150|100|1".to_string()]
    );
}

#[test]
fn a_reading_is_asked_the_same_question() {
    let layout = Layout::parse();
    let readings = [ChdefReading {
        channel: 1,
        raw: 150,
        value: 150.0,
    }];

    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe {
            chdef_readings_out_of_range(layout.0, readings.as_ptr(), readings.len(), &mut issues)
        },
        CHDEF_OK
    );
    assert_eq!(
        described(issues),
        vec!["value_out_of_range|150|100|1".to_string()]
    );
}

#[test]
fn asking_changes_nothing_about_what_is_written() {
    let layout = Layout::parse();
    let values = [ChdefValue::physical(1, 150.0)];

    let encode = || {
        let mut frame = [0u8; 4];
        let mut len = 0usize;
        let mut issues = ptr::null_mut();
        assert_eq!(
            unsafe {
                chdef_encode(
                    layout.0,
                    values.as_ptr(),
                    values.len(),
                    frame.as_mut_ptr(),
                    frame.len(),
                    &mut len,
                    &mut issues,
                )
            },
            CHDEF_OK
        );
        let count = unsafe { chdef_issue_count(issues) };
        unsafe { chdef_issues_free(issues) };
        (frame, count)
    };

    let before = encode();
    assert!(!values_out_of_range(&layout, &values).is_empty());
    let after = encode();

    assert_eq!(before.0, after.0, "the frame is untouched");
    assert_eq!(before.1, 0, "encode still says nothing");
    assert_eq!(after.1, 0);
}

#[test]
fn an_empty_ask_is_not_an_error() {
    let layout = Layout::parse();
    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_values_out_of_range(layout.0, ptr::null(), 0, &mut issues) },
        CHDEF_OK
    );
    assert_eq!(unsafe { chdef_issue_count(issues) }, 0);
    unsafe { chdef_issues_free(issues) };
}

#[test]
fn an_unusable_handle_is_reported() {
    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_values_out_of_range(ptr::null(), ptr::null(), 0, &mut issues) },
        CHDEF_ERR_HANDLE
    );
    assert_eq!(
        unsafe { chdef_readings_out_of_range(ptr::null(), ptr::null(), 0, &mut issues) },
        CHDEF_ERR_HANDLE
    );
}

// ------------------------------------------------------ the declared range

fn range_of(layout: &Layout, index: usize) -> Result<ChdefRange, i32> {
    let mut range = ChdefRange::default();
    match unsafe { chdef_layout_channel_range(layout.0, index, &mut range) } {
        CHDEF_OK => Ok(range),
        status => Err(status),
    }
}

#[test]
fn a_declared_range_crosses_as_physical_values_with_each_side_marked_present() {
    // conversion.md §8: the bounds are physical, resolved with the row's
    // lsb and offset.
    let layout = Layout::parse();
    let declared = range_of(&layout, 0).unwrap();
    assert_eq!((declared.has_min, declared.min), (1, 0.0));
    assert_eq!((declared.has_max, declared.max), (1, 100.0));
}

#[test]
fn an_unspecified_side_crosses_as_absent() {
    let layout = Layout::parse();
    let declared = range_of(&layout, 1).unwrap();
    assert_eq!(declared.has_min, 0);
    assert_eq!(declared.has_max, 0);
}

#[test]
fn a_bound_given_as_a_raw_pattern_is_resolved_before_it_crosses() {
    let text = "number,bytes,type,lsb,offset,min,max\n1,2,UI,0.5,-10,0x10,\n";
    let mut layout = ptr::null_mut();
    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe {
            chdef_layout_parse(
                text.as_ptr(),
                text.len(),
                ptr::null(),
                0,
                &mut layout,
                &mut issues,
                ptr::null_mut(),
                0,
            )
        },
        CHDEF_OK
    );
    unsafe { chdef_issues_free(issues) };
    let layout = Layout(layout);

    let declared = range_of(&layout, 0).unwrap();
    assert_eq!((declared.has_min, declared.min), (1, 16.0 * 0.5 - 10.0));
    assert_eq!(declared.has_max, 0);
}

#[test]
fn a_range_index_outside_the_layout_is_an_index_error() {
    let layout = Layout::parse();
    assert_eq!(range_of(&layout, 9), Err(CHDEF_ERR_INDEX));
    assert_eq!(
        unsafe { chdef_layout_channel_range(layout.0, 0, ptr::null_mut()) },
        CHDEF_ERR_NULL
    );
}
