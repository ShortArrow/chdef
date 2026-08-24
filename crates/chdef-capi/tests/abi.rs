//! The C ABI, exercised the way a C caller does. `docs/spec/interchange.md`
//! §3's vectors run through it separately (`vectors_through_the_abi.rs`),
//! so these tests are about the boundary itself: statuses, buffers,
//! handles, and the fields that cross it.

use std::ffi::c_char;
use std::ptr;

use chdef_capi::*;

const CH: &str = "number,bytes,type,name,lsb,offset,unit,default\n\
                  1,4,UI,Frame,1,0,,\n\
                  2,2,BF,Status,1,0,,0x0005\n\
                  3,2,SI,Temp,0.1,-40,degC,\n";
const BF: &str = "number,bit,name,default\n2,0,alive,\n2,2,fault,0\n";

/// Read a `chdef_*_text` field into a `String` the way a C caller would:
/// ask for the length, then fill a buffer of that size.
fn text(mut fill: impl FnMut(*mut c_char, usize) -> usize) -> String {
    let needed = fill(ptr::null_mut(), 0);
    let mut buf = vec![0u8; needed + 1];
    let written = fill(buf.as_mut_ptr() as *mut c_char, buf.len());
    assert_eq!(written, needed, "the length query and the fill disagree");
    buf.truncate(written);
    String::from_utf8(buf).unwrap()
}

fn parse(ch: &str, bf: &str) -> (*mut ChdefLayout, *mut ChdefIssues) {
    let mut layout = ptr::null_mut();
    let mut issues = ptr::null_mut();
    let status = unsafe {
        chdef_layout_parse(
            ch.as_ptr(),
            ch.len(),
            bf.as_ptr(),
            bf.len(),
            &mut layout,
            &mut issues,
            ptr::null_mut(),
            0,
        )
    };
    assert_eq!(status, CHDEF_OK, "parse failed");
    assert!(!layout.is_null() && !issues.is_null());
    (layout, issues)
}

#[test]
fn the_abi_version_is_readable_before_anything_else() {
    assert!(chdef_abi_version() >= 1);
}

#[test]
fn a_definition_set_parses_into_a_layout() {
    let (layout, issues) = parse(CH, BF);

    assert_eq!(unsafe { chdef_layout_total_bytes(layout) }, 8);
    assert_eq!(unsafe { chdef_layout_channel_count(layout) }, 3);
    assert_eq!(unsafe { chdef_issue_count(issues) }, 0);

    unsafe {
        chdef_issues_free(issues);
        chdef_layout_free(layout);
    }
}

#[test]
fn a_channel_describes_itself_in_numbers_and_strings() {
    let (layout, issues) = parse(CH, BF);

    let mut ch = ChdefChannel::default();
    assert_eq!(
        unsafe { chdef_layout_channel_at(layout, 2, &mut ch) },
        CHDEF_OK
    );
    assert_eq!((ch.number, ch.at, ch.bytes), (3, 6, 2));
    assert_eq!(ch.lsb, 0.1);
    assert_eq!(ch.offset, -40.0);
    assert_eq!(ch.default_value, -1, "channel 3 states no default");

    let name = text(|buf, cap| unsafe {
        chdef_layout_channel_text(layout, 2, CHDEF_CHANNEL_NAME, buf, cap)
    });
    assert_eq!(name, "Temp");
    let ty = text(|buf, cap| unsafe {
        chdef_layout_channel_text(layout, 2, CHDEF_CHANNEL_TYPE, buf, cap)
    });
    assert_eq!(ty, "SI", "the interpretation crosses as its stable string");
    let unit = text(|buf, cap| unsafe {
        chdef_layout_channel_text(layout, 2, CHDEF_CHANNEL_UNIT, buf, cap)
    });
    assert_eq!(unit, "degC");

    unsafe {
        chdef_issues_free(issues);
        chdef_layout_free(layout);
    }
}

#[test]
fn a_default_that_exists_crosses_as_a_non_negative_number() {
    let (layout, issues) = parse(CH, BF);

    let mut ch = ChdefChannel::default();
    unsafe { chdef_layout_channel_at(layout, 1, &mut ch) };

    // The BF rows fold into it: 0x0005 with bit 2 cleared is 0x0001.
    assert_eq!(ch.default_value, 1);

    unsafe {
        chdef_issues_free(issues);
        chdef_layout_free(layout);
    }
}

#[test]
fn a_frame_encodes_and_decodes_through_the_boundary() {
    let (layout, issues) = parse(CH, BF);
    unsafe { chdef_issues_free(issues) };

    let values = [ChdefValue::physical(1, 7.0), ChdefValue::physical(3, -12.3)];
    let mut frame = [0u8; 8];
    let mut written = 0usize;
    let mut encode_issues = ptr::null_mut();
    let status = unsafe {
        chdef_encode(
            layout,
            values.as_ptr(),
            values.len(),
            frame.as_mut_ptr(),
            frame.len(),
            &mut written,
            &mut encode_issues,
        )
    };
    assert_eq!(status, CHDEF_OK);
    assert_eq!(written, 8);
    assert_eq!(unsafe { chdef_issue_count(encode_issues) }, 0);
    unsafe { chdef_issues_free(encode_issues) };

    let mut readings = [ChdefReading::default(); 8];
    let mut count = 0usize;
    let status = unsafe {
        chdef_decode(
            layout,
            frame.as_ptr(),
            frame.len(),
            readings.as_mut_ptr(),
            readings.len(),
            &mut count,
        )
    };
    assert_eq!(status, CHDEF_OK);
    assert_eq!(count, 3);
    assert_eq!(readings[0].channel, 1);
    assert_eq!(readings[0].value, 7.0);
    assert!((readings[2].value - -12.3).abs() < 1e-9);

    unsafe { chdef_layout_free(layout) };
}

#[test]
fn a_raw_value_crosses_as_a_raw_value() {
    let (layout, issues) = parse(CH, BF);
    unsafe { chdef_issues_free(issues) };

    let values = [ChdefValue::raw(1, 0xDEAD_BEEF)];
    let mut frame = [0u8; 8];
    let mut written = 0usize;
    let mut encode_issues = ptr::null_mut();
    unsafe {
        chdef_encode(
            layout,
            values.as_ptr(),
            values.len(),
            frame.as_mut_ptr(),
            frame.len(),
            &mut written,
            &mut encode_issues,
        )
    };
    unsafe { chdef_issues_free(encode_issues) };

    assert_eq!(&frame[..4], &[0xEF, 0xBE, 0xAD, 0xDE]);
    unsafe { chdef_layout_free(layout) };
}

#[test]
fn the_byte_order_of_the_layout_is_settable() {
    let (layout, issues) = parse(CH, BF);
    unsafe { chdef_issues_free(issues) };

    assert_eq!(
        unsafe { chdef_layout_set_endian(layout, CHDEF_BIG) },
        CHDEF_OK
    );

    let values = [ChdefValue::raw(1, 1)];
    let mut frame = [0u8; 8];
    let mut written = 0usize;
    let mut encode_issues = ptr::null_mut();
    unsafe {
        chdef_encode(
            layout,
            values.as_ptr(),
            values.len(),
            frame.as_mut_ptr(),
            frame.len(),
            &mut written,
            &mut encode_issues,
        )
    };
    unsafe { chdef_issues_free(encode_issues) };

    assert_eq!(&frame[..4], &[0x00, 0x00, 0x00, 0x01]);
    unsafe { chdef_layout_free(layout) };
}

#[test]
fn an_issue_crosses_as_its_stable_code_and_its_values() {
    let (layout, issues) = parse("number,bytes,name\n1,99,a\n", "");

    assert_eq!(unsafe { chdef_issue_count(issues) }, 1);
    let mut issue = ChdefIssue::default();
    assert_eq!(unsafe { chdef_issue_at(issues, 0, &mut issue) }, CHDEF_OK);
    assert_eq!(issue.row, 0);
    assert_eq!(issue.channel, 1);
    assert_eq!(issue.bit, -1, "a channel issue names no bit");

    let code = text(|buf, cap| unsafe { chdef_issue_text(issues, 0, CHDEF_ISSUE_CODE, buf, cap) });
    assert_eq!(code, "bytes_out_of_range");
    let found =
        text(|buf, cap| unsafe { chdef_issue_text(issues, 0, CHDEF_ISSUE_FOUND, buf, cap) });
    assert_eq!(found, "99");
    let used = text(|buf, cap| unsafe { chdef_issue_text(issues, 0, CHDEF_ISSUE_USED, buf, cap) });
    assert_eq!(used, "8");

    unsafe {
        chdef_issues_free(issues);
        chdef_layout_free(layout);
    }
}

#[test]
fn a_broken_file_reports_its_error_into_the_callers_buffer() {
    let broken = "number,name\n1,\"never closed\n2,b\n";
    let mut layout = ptr::null_mut();
    let mut issues = ptr::null_mut();
    let mut err = [0u8; 256];

    let status = unsafe {
        chdef_layout_parse(
            broken.as_ptr(),
            broken.len(),
            ptr::null(),
            0,
            &mut layout,
            &mut issues,
            err.as_mut_ptr() as *mut c_char,
            err.len(),
        )
    };

    assert_eq!(status, CHDEF_ERR_CSV);
    assert!(layout.is_null() && issues.is_null());
    let message = String::from_utf8(err.iter().copied().take_while(|b| *b != 0).collect()).unwrap();
    assert!(message.contains("quoted cell"), "got {message:?}");
}

#[test]
fn a_text_field_that_does_not_fit_is_truncated_and_says_what_it_needed() {
    let (layout, issues) = parse(CH, BF);

    let mut small = [0u8; 3];
    let needed = unsafe {
        chdef_layout_channel_text(
            layout,
            0,
            CHDEF_CHANNEL_NAME,
            small.as_mut_ptr() as *mut c_char,
            small.len(),
        )
    };

    assert_eq!(needed, "Frame".len());
    assert_eq!(&small, b"Fr\0", "truncated, and still terminated");

    unsafe {
        chdef_issues_free(issues);
        chdef_layout_free(layout);
    }
}

#[test]
fn a_null_handle_is_a_status_not_a_crash() {
    let mut ch = ChdefChannel::default();

    assert_eq!(unsafe { chdef_layout_total_bytes(ptr::null()) }, 0);
    assert_eq!(unsafe { chdef_layout_channel_count(ptr::null()) }, 0);
    assert_eq!(
        unsafe { chdef_layout_channel_at(ptr::null(), 0, &mut ch) },
        CHDEF_ERR_HANDLE
    );
    assert_eq!(unsafe { chdef_issue_count(ptr::null()) }, 0);
    unsafe {
        chdef_layout_free(ptr::null_mut());
        chdef_issues_free(ptr::null_mut());
    }
}

#[test]
fn an_index_past_the_end_is_a_status_not_a_crash() {
    let (layout, issues) = parse(CH, BF);
    let mut ch = ChdefChannel::default();

    assert_eq!(
        unsafe { chdef_layout_channel_at(layout, 99, &mut ch) },
        CHDEF_ERR_INDEX
    );
    let mut issue = ChdefIssue::default();
    assert_eq!(
        unsafe { chdef_issue_at(issues, 0, &mut issue) },
        CHDEF_ERR_INDEX
    );

    unsafe {
        chdef_issues_free(issues);
        chdef_layout_free(layout);
    }
}

#[test]
fn a_frame_buffer_too_small_is_a_status_not_a_write_past_the_end() {
    let (layout, issues) = parse(CH, BF);
    unsafe { chdef_issues_free(issues) };

    let mut frame = [0u8; 2];
    let mut written = 0usize;
    let mut encode_issues = ptr::null_mut();
    let status = unsafe {
        chdef_encode(
            layout,
            ptr::null(),
            0,
            frame.as_mut_ptr(),
            frame.len(),
            &mut written,
            &mut encode_issues,
        )
    };

    assert_eq!(status, CHDEF_ERR_BUFFER);
    assert_eq!(written, 8, "how many bytes the frame needs");
    assert_eq!(frame, [0, 0], "nothing was written");

    unsafe { chdef_layout_free(layout) };
}

#[test]
fn the_capacity_of_a_layout_is_checked_only_when_stated() {
    let (layout, issues) = parse(CH, BF);
    unsafe { chdef_issues_free(issues) };

    let mut over = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_layout_check_capacity(layout, &mut over) },
        CHDEF_OK
    );
    assert_eq!(unsafe { chdef_issue_count(over) }, 0, "no capacity stated");
    unsafe { chdef_issues_free(over) };

    unsafe { chdef_layout_set_capacity(layout, 4) };
    let mut over = ptr::null_mut();
    unsafe { chdef_layout_check_capacity(layout, &mut over) };
    assert_eq!(unsafe { chdef_issue_count(over) }, 1);
    let code = text(|buf, cap| unsafe { chdef_issue_text(over, 0, CHDEF_ISSUE_CODE, buf, cap) });
    assert_eq!(code, "layout_exceeds_capacity");

    unsafe {
        chdef_issues_free(over);
        chdef_layout_free(layout);
    }
}
