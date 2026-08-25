//! Derived channels across the C ABI (`docs/spec/format.md` §6): sealing,
//! checking, and the coverage a caller computing its own checksum needs.

use std::ffi::c_char;
use std::ptr;

use chdef_capi::*;

/// Sync, counter, payload, then a CRC over channels 1..3. The expected
/// value comes from a separate implementation of the Rocksoft model.
const CH: &str = "number,bytes,type,kind,derived,default,name\n\
                  1,2,UI,const,,0x7E7E,SYNC\n\
                  2,2,UI,counter,,1,FRAME_NO\n\
                  3,1,UI,plain,,42,PAYLOAD\n\
                  4,2,UI,derived,crc16/x25 1..3,,CRC\n";

const BODY: [u8; 5] = [0x7E, 0x7E, 0x01, 0x00, 0x2A];
const CRC_OVER_1_3: u16 = 0x9BBF;

fn text(mut call: impl FnMut(*mut c_char, usize) -> usize) -> String {
    let needed = call(ptr::null_mut(), 0);
    let mut buf = vec![0u8; needed + 1];
    call(buf.as_mut_ptr() as *mut c_char, buf.len());
    buf.truncate(needed);
    String::from_utf8(buf).unwrap()
}

struct Layout(*mut ChdefLayout);

impl Layout {
    fn parse(ch: &str) -> Layout {
        let mut layout = ptr::null_mut();
        let mut issues = ptr::null_mut();
        assert_eq!(
            unsafe {
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
            },
            CHDEF_OK
        );
        unsafe { chdef_issues_free(issues) };
        Layout(layout)
    }

    fn encoded(&self) -> Vec<u8> {
        let mut frame = vec![0u8; unsafe { chdef_layout_total_bytes(self.0) } as usize];
        let mut len = 0usize;
        let mut issues = ptr::null_mut();
        assert_eq!(
            unsafe {
                chdef_encode(
                    self.0,
                    ptr::null(),
                    0,
                    frame.as_mut_ptr(),
                    frame.len(),
                    &mut len,
                    &mut issues,
                )
            },
            CHDEF_OK
        );
        unsafe { chdef_issues_free(issues) };
        frame.truncate(len);
        frame
    }
}

impl Drop for Layout {
    fn drop(&mut self) {
        unsafe { chdef_layout_free(self.0) };
    }
}

fn codes(issues: *mut ChdefIssues) -> Vec<String> {
    let listed = (0..unsafe { chdef_issue_count(issues) } as usize)
        .map(|i| {
            text(|buf, cap| unsafe { chdef_issue_text(issues, i, CHDEF_ISSUE_CODE, buf, cap) })
        })
        .collect();
    unsafe { chdef_issues_free(issues) };
    listed
}

fn seal(layout: &Layout, frame: &mut [u8]) -> Vec<String> {
    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_seal(layout.0, frame.as_mut_ptr(), frame.len(), &mut issues) },
        CHDEF_OK
    );
    codes(issues)
}

// --------------------------------------------------------------- sealing

#[test]
fn encode_leaves_the_derived_channel_and_sealing_fills_it() {
    let layout = Layout::parse(CH);
    let mut frame = layout.encoded();

    assert_eq!(&frame[..5], &BODY);
    assert_eq!(&frame[5..], &[0, 0], "encode does not compute it");

    assert!(seal(&layout, &mut frame).is_empty());
    assert_eq!(&frame[5..], &CRC_OVER_1_3.to_le_bytes());
    assert_eq!(&frame[..5], &BODY, "nothing else moved");
}

#[test]
fn the_recipe_cell_crosses_as_the_file_spells_it() {
    let layout = Layout::parse(CH);
    let field = |index, which| {
        text(|buf, cap| unsafe { chdef_layout_channel_text(layout.0, index, which, buf, cap) })
    };

    assert_eq!(field(3, CHDEF_CHANNEL_KIND), "derived");
    assert_eq!(field(3, CHDEF_CHANNEL_DERIVED), "crc16/x25 1..3");
    assert_eq!(field(0, CHDEF_CHANNEL_DERIVED), "", "not a derived channel");
}

// -------------------------------------------------------------- checking

#[test]
fn a_sealed_frame_checks_out_and_an_unsealed_one_does_not() {
    let layout = Layout::parse(CH);
    let mut frame = layout.encoded();

    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_derived_mismatches(layout.0, frame.as_ptr(), frame.len(), &mut issues) },
        CHDEF_OK
    );
    assert_eq!(codes(issues), vec!["derived_mismatch".to_string()]);

    seal(&layout, &mut frame);
    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe { chdef_derived_mismatches(layout.0, frame.as_ptr(), frame.len(), &mut issues) },
        CHDEF_OK
    );
    assert!(codes(issues).is_empty());
}

// --------------------------------------------------- the storey below

#[test]
fn the_coverage_crosses_even_when_the_recipe_does_not() {
    // ADR-0029: a device whose checksum chdef does not compute is not
    // blocked — the bytes it covers are what chdef alone can say.
    let layout = Layout::parse(
        "number,bytes,type,kind,derived,default,name\n\
         1,2,UI,const,,0x7E7E,SYNC\n\
         2,2,UI,counter,,1,FRAME_NO\n\
         3,1,UI,plain,,42,PAYLOAD\n\
         4,2,UI,derived,fletcher16 1..3,,SUM\n",
    );
    let mut frame = layout.encoded();

    assert_eq!(
        seal(&layout, &mut frame),
        vec!["derived_unknown_recipe".to_string()]
    );
    assert_eq!(&frame[5..], &[0, 0], "and nothing was computed");

    let mut needed = 0usize;
    assert_eq!(
        unsafe {
            chdef_covered_bytes(
                layout.0,
                4,
                frame.as_ptr(),
                frame.len(),
                ptr::null_mut(),
                0,
                &mut needed,
            )
        },
        CHDEF_ERR_BUFFER,
        "the two-call pattern reports the length first"
    );
    assert_eq!(needed, BODY.len());

    let mut covered = vec![0u8; needed];
    assert_eq!(
        unsafe {
            chdef_covered_bytes(
                layout.0,
                4,
                frame.as_ptr(),
                frame.len(),
                covered.as_mut_ptr(),
                covered.len(),
                &mut needed,
            )
        },
        CHDEF_OK
    );
    assert_eq!(covered, BODY);
}

#[test]
fn a_channel_with_no_coverage_to_give_is_an_index_error() {
    let layout = Layout::parse(CH);
    let frame = layout.encoded();
    let mut needed = 0usize;

    for channel in [1u32, 9] {
        assert_eq!(
            unsafe {
                chdef_covered_bytes(
                    layout.0,
                    channel,
                    frame.as_ptr(),
                    frame.len(),
                    ptr::null_mut(),
                    0,
                    &mut needed,
                )
            },
            CHDEF_ERR_INDEX,
            "channel {channel}"
        );
        assert_eq!(needed, 0);
    }
}

// ------------------------------------------------------ the vocabulary

#[test]
fn the_recipes_this_library_knows_are_enumerable() {
    let count = chdef_recipe_count() as usize;
    assert!(count >= 6, "found only {count}");

    let names: Vec<String> = (0..count)
        .map(|i| text(|buf, cap| unsafe { chdef_recipe_name(i, buf, cap) }))
        .collect();
    assert!(names.contains(&"crc16/x25".to_string()), "{names:?}");
    assert_eq!(
        text(|buf, cap| unsafe { chdef_recipe_name(count, buf, cap) }),
        "",
        "past the end"
    );
}

#[test]
fn an_unusable_handle_is_reported() {
    let mut issues = ptr::null_mut();
    let mut frame = [0u8; 4];
    assert_eq!(
        unsafe { chdef_seal(ptr::null(), frame.as_mut_ptr(), frame.len(), &mut issues) },
        CHDEF_ERR_HANDLE
    );
    assert_eq!(
        unsafe { chdef_derived_mismatches(ptr::null(), frame.as_ptr(), frame.len(), &mut issues) },
        CHDEF_ERR_HANDLE
    );
    let mut needed = 0usize;
    assert_eq!(
        unsafe {
            chdef_covered_bytes(
                ptr::null(),
                1,
                frame.as_ptr(),
                frame.len(),
                ptr::null_mut(),
                0,
                &mut needed,
            )
        },
        CHDEF_ERR_HANDLE
    );
}

#[test]
fn the_version_rose_when_the_surface_grew() {
    assert!(chdef_abi_version() >= 6);
}
