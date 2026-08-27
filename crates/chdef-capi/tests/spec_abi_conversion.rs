//! Converting one value either way across the C ABI, and asking whether a
//! width holds it (`docs/spec/conversion.md` §1–§2). `chdef_encode` and
//! `chdef_decode` carry the same rules for a whole frame; a consumer
//! showing one cell wants them for one number, and without these calls
//! would write the arithmetic a second time (ADR-0023).

use std::ptr;

use chdef_capi::*;

const CH: &str = "number,bytes,type,lsb,offset\n\
                  1,1,UI,1,0\n\
                  2,1,SI,0.5,-10\n\
                  3,2,UI,0,0\n";

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

fn to_raw(layout: &Layout, index: usize, value: f64) -> Result<u64, i32> {
    let mut raw = 0u64;
    match unsafe { chdef_layout_channel_to_raw(layout.0, index, value, &mut raw) } {
        CHDEF_OK => Ok(raw),
        status => Err(status),
    }
}

fn to_value(layout: &Layout, index: usize, raw: u64) -> Result<f64, i32> {
    let mut value = 0f64;
    match unsafe { chdef_layout_channel_to_value(layout.0, index, raw, &mut value) } {
        CHDEF_OK => Ok(value),
        status => Err(status),
    }
}

fn fits_width(layout: &Layout, index: usize, value: f64) -> Result<bool, i32> {
    let mut fits = -1i32;
    match unsafe { chdef_layout_channel_fits_width(layout.0, index, value, &mut fits) } {
        CHDEF_OK => Ok(fits != 0),
        status => Err(status),
    }
}

// ------------------------------------------------------- physical → raw

#[test]
fn a_physical_value_rounds_half_away_from_zero() {
    // conversion.md §2: 0.5 → 1, 2.5 → 3.
    let layout = Layout::parse();
    assert_eq!(to_raw(&layout, 0, 0.5), Ok(1));
    assert_eq!(to_raw(&layout, 0, 2.5), Ok(3));
    assert_eq!(to_raw(&layout, 1, -0.25 - 10.0), Ok(0xFF), "−0.5 → −1");
}

#[test]
fn a_value_the_width_cannot_hold_is_clamped_to_it() {
    // conversion.md §2: UI clamps to [0, 2^bits − 1].
    let layout = Layout::parse();
    assert_eq!(to_raw(&layout, 0, 300.0), Ok(255));
    assert_eq!(to_raw(&layout, 0, -3.0), Ok(0));
}

#[test]
fn a_negative_signed_value_becomes_its_twos_complement_pattern() {
    // conversion.md §2: −20 physical at lsb 0.5, offset −10 is raw −20.
    let layout = Layout::parse();
    assert_eq!(to_raw(&layout, 1, -20.0), Ok(0xEC));
}

#[test]
fn a_zero_lsb_counts_as_one() {
    let layout = Layout::parse();
    assert_eq!(to_raw(&layout, 2, 7.0), Ok(7));
}

#[test]
fn a_value_that_cannot_be_converted_is_a_value_error() {
    // conversion.md §2: a NaN / infinite value cannot be converted.
    let layout = Layout::parse();
    assert_eq!(to_raw(&layout, 0, f64::NAN), Err(CHDEF_ERR_VALUE));
    assert_eq!(to_raw(&layout, 0, f64::INFINITY), Err(CHDEF_ERR_VALUE));
}

// ------------------------------------------------------- raw → physical

#[test]
fn a_raw_pattern_is_scaled_and_offset() {
    // conversion.md §1: value = raw_signed × lsb + offset.
    let layout = Layout::parse();
    assert_eq!(to_value(&layout, 0, 7), Ok(7.0));
    assert_eq!(
        to_value(&layout, 1, 0xEC),
        Ok(-20.0),
        "sign-extended for SI"
    );
    assert_eq!(to_value(&layout, 2, 5), Ok(5.0), "lsb 0 is 1");
}

#[test]
fn bits_beyond_the_width_are_ignored() {
    // conversion.md §1: bits beyond the width are ignored.
    let layout = Layout::parse();
    assert_eq!(to_value(&layout, 0, 0x1_07), Ok(7.0));
}

#[test]
fn the_two_conversions_agree_with_encode_and_decode() {
    let layout = Layout::parse();
    let raw = to_raw(&layout, 1, -20.0).unwrap();
    let mut frame = [0u8; 4];
    let mut len = 0usize;
    let mut issues = ptr::null_mut();
    let values = [ChdefValue::physical(2, -20.0)];
    assert_eq!(
        unsafe {
            chdef_encode(
                layout.0,
                values.as_ptr(),
                1,
                frame.as_mut_ptr(),
                frame.len(),
                &mut len,
                &mut issues,
            )
        },
        CHDEF_OK
    );
    unsafe { chdef_issues_free(issues) };
    assert_eq!(frame[1] as u64, raw);
    assert_eq!(to_value(&layout, 1, frame[1] as u64), Ok(-20.0));
}

// ------------------------------------------------------------ the ask

#[test]
fn whether_the_width_holds_a_value_is_answered_without_converting_it() {
    // conversion.md §2: fits_width is false when the width would clamp.
    let layout = Layout::parse();
    assert_eq!(fits_width(&layout, 0, 255.0), Ok(true));
    assert_eq!(fits_width(&layout, 0, 256.0), Ok(false));
    assert_eq!(fits_width(&layout, 0, -1.0), Ok(false));
    assert_eq!(fits_width(&layout, 0, f64::NAN), Ok(false));
}

// ------------------------------------------------------------ boundary

#[test]
fn an_index_outside_the_layout_is_an_index_error() {
    let layout = Layout::parse();
    assert_eq!(to_raw(&layout, 9, 1.0), Err(CHDEF_ERR_INDEX));
    assert_eq!(to_value(&layout, 9, 1), Err(CHDEF_ERR_INDEX));
    assert_eq!(fits_width(&layout, 9, 1.0), Err(CHDEF_ERR_INDEX));
}

#[test]
fn a_null_out_pointer_is_a_null_error_and_nothing_is_read() {
    let layout = Layout::parse();
    assert_eq!(
        unsafe { chdef_layout_channel_to_raw(layout.0, 0, 1.0, ptr::null_mut()) },
        CHDEF_ERR_NULL
    );
    assert_eq!(
        unsafe { chdef_layout_channel_to_value(layout.0, 0, 1, ptr::null_mut()) },
        CHDEF_ERR_NULL
    );
    assert_eq!(
        unsafe { chdef_layout_channel_fits_width(layout.0, 0, 1.0, ptr::null_mut()) },
        CHDEF_ERR_NULL
    );
}
