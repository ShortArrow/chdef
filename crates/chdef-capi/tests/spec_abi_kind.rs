//! The 0.0.8 surface across the C ABI: who fills a channel (ADR-0025), the
//! limits a layout is measured against, and the Issue codes a caller
//! checks its own table against (ADR-0026).

use std::ffi::c_char;
use std::ptr;

use chdef_capi::*;

const CH: &str = "number,bytes,type,kind,default,name\n\
                  1,2,UI,const,0x7E7E,SYNC\n\
                  2,2,UI,counter,,FRAME_NO\n\
                  3,1,UI,,,PAYLOAD\n";

fn text(mut call: impl FnMut(*mut c_char, usize) -> usize) -> String {
    let needed = call(ptr::null_mut(), 0);
    let mut buf = vec![0u8; needed + 1];
    call(buf.as_mut_ptr() as *mut c_char, buf.len());
    buf.truncate(needed);
    String::from_utf8(buf).unwrap()
}

struct Layout(*mut ChdefLayout, *mut ChdefIssues);

impl Layout {
    fn parse(ch: &str) -> Layout {
        let mut layout = ptr::null_mut();
        let mut issues = ptr::null_mut();
        let status = unsafe {
            chdef_layout_parse(
                ch.as_ptr(),
                ch.len(),
                ptr::null(),
                0,
                &mut layout,
                &mut issues,
                ptr::null_mut(),
                0,
            )
        };
        assert_eq!(status, CHDEF_OK);
        Layout(layout, issues)
    }

    fn field(&self, index: usize, which: i32) -> String {
        text(|buf, cap| unsafe { chdef_layout_channel_text(self.0, index, which, buf, cap) })
    }
}

impl Drop for Layout {
    fn drop(&mut self) {
        unsafe {
            chdef_issues_free(self.1);
            chdef_layout_free(self.0);
        }
    }
}

// ------------------------------------------------------------------- kind

#[test]
fn the_kind_of_each_channel_crosses_as_its_own_string() {
    let layout = Layout::parse(CH);

    assert_eq!(layout.field(0, CHDEF_CHANNEL_KIND), "const");
    assert_eq!(layout.field(1, CHDEF_CHANNEL_KIND), "counter");
    assert_eq!(layout.field(2, CHDEF_CHANNEL_KIND), "plain");
}

#[test]
fn kind_is_a_field_of_its_own_and_not_another_one_over_again() {
    // The field selectors are distinct: a channel with a `format` and a
    // `kind` that differ must answer differently.
    let layout = Layout::parse("number,bytes,format,kind\n1,2,HEX,counter\n");

    assert_eq!(layout.field(0, CHDEF_CHANNEL_FORMAT), "HEX");
    assert_eq!(layout.field(0, CHDEF_CHANNEL_KIND), "counter");
    assert_eq!(layout.field(0, CHDEF_CHANNEL_NAME), "");
}

#[test]
fn a_kind_the_library_does_not_know_crosses_as_plain_with_a_finding() {
    let layout = Layout::parse("number,bytes,kind\n1,2,derived\n");

    assert_eq!(layout.field(0, CHDEF_CHANNEL_KIND), "plain");
    let codes: Vec<String> = (0..unsafe { chdef_issue_count(layout.1) } as usize)
        .map(|i| {
            text(|buf, cap| unsafe { chdef_issue_text(layout.1, i, CHDEF_ISSUE_CODE, buf, cap) })
        })
        .collect();
    assert!(
        codes.iter().any(|c| c == "kind_assumed"),
        "expected kind_assumed, got {codes:?}"
    );
}

// ----------------------------------------------------------------- limits

#[test]
fn both_limits_are_stated_and_both_are_reported() {
    let layout = Layout::parse(CH);
    assert_eq!(unsafe { chdef_layout_set_capacity(layout.0, 2) }, CHDEF_OK);
    assert_eq!(
        unsafe { chdef_layout_set_channel_capacity(layout.0, 1) },
        CHDEF_OK
    );

    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_layout_limits_exceeded(layout.0, &mut issues) },
        CHDEF_OK
    );
    let codes: Vec<String> = (0..unsafe { chdef_issue_count(issues) } as usize)
        .map(|i| {
            text(|buf, cap| unsafe { chdef_issue_text(issues, i, CHDEF_ISSUE_CODE, buf, cap) })
        })
        .collect();
    unsafe { chdef_issues_free(issues) };

    assert_eq!(
        codes,
        vec![
            "layout_exceeds_capacity".to_string(),
            "layout_exceeds_channel_capacity".to_string()
        ]
    );
}

#[test]
fn a_layout_within_both_limits_reports_nothing() {
    let layout = Layout::parse(CH);
    assert_eq!(
        unsafe { chdef_layout_set_capacity(layout.0, 246) },
        CHDEF_OK
    );
    assert_eq!(
        unsafe { chdef_layout_set_channel_capacity(layout.0, 64) },
        CHDEF_OK
    );

    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_layout_limits_exceeded(layout.0, &mut issues) },
        CHDEF_OK
    );
    assert_eq!(unsafe { chdef_issue_count(issues) }, 0);
    unsafe { chdef_issues_free(issues) };
}

#[test]
fn setting_a_limit_on_an_unusable_handle_is_reported() {
    assert_eq!(
        unsafe { chdef_layout_set_channel_capacity(ptr::null_mut(), 64) },
        CHDEF_ERR_HANDLE
    );
}

// ------------------------------------------------------------ issue codes

#[test]
fn every_issue_code_is_reachable_across_the_boundary() {
    // ADR-0026: where chdef says new codes may appear, it says what they
    // are today.
    let count = chdef_issue_code_count() as usize;
    assert!(count >= 26, "found only {count}");

    let codes: Vec<String> = (0..count)
        .map(|i| text(|buf, cap| unsafe { chdef_issue_code_name(i, buf, cap) }))
        .collect();

    assert!(codes.contains(&"header_assumed".to_string()));
    assert!(codes.contains(&"kind_assumed".to_string()));
    assert!(codes.contains(&"layout_exceeds_channel_capacity".to_string()));
    assert_eq!(
        text(|buf, cap| unsafe { chdef_issue_code_name(count, buf, cap) }),
        "",
        "past the end"
    );
}

#[test]
fn the_count_is_the_length_of_the_list_it_describes() {
    // The floor above would still pass if the count over-reported; this is
    // what catches a count that does not match what can be read.
    let count = chdef_issue_code_count() as usize;
    for index in 0..count {
        assert!(
            !text(|buf, cap| unsafe { chdef_issue_code_name(index, buf, cap) }).is_empty(),
            "code {index} of {count} reads as nothing"
        );
    }
    assert_eq!(
        text(|buf, cap| unsafe { chdef_issue_code_name(count, buf, cap) }),
        "",
        "the list ends exactly where the count says"
    );
}

#[test]
fn a_code_that_arrives_is_one_the_list_holds() {
    let layout = Layout::parse("number,bytes,kind\n1,2,derived\n");
    let listed: Vec<String> = (0..chdef_issue_code_count() as usize)
        .map(|i| text(|buf, cap| unsafe { chdef_issue_code_name(i, buf, cap) }))
        .collect();

    let count = unsafe { chdef_issue_count(layout.1) } as usize;
    assert!(count > 0);
    for i in 0..count {
        let code =
            text(|buf, cap| unsafe { chdef_issue_text(layout.1, i, CHDEF_ISSUE_CODE, buf, cap) });
        assert!(listed.contains(&code), "{code} is not in the list");
    }
}

#[test]
fn the_version_rose_when_the_surface_grew() {
    assert!(chdef_abi_version() >= 4);
}
