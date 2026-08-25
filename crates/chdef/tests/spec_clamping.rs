//! A value the width cannot hold (`docs/spec/conversion.md` §2).
//!
//! The clamping is right — a reading past the end of a sensor's range
//! saturates, which is what belongs on the wire. What was wrong was that
//! chdef said nothing about it, while every other lossy reading of a cell
//! reports one (`bytes_out_of_range`, `raw_out_of_range`, `min_max_swapped`).

use chdef::*;

fn layout(bytes: usize, data_type: DataType, lsb: f64, offset: f64) -> ChannelLayout {
    let mut def = ChannelDef::new(1, bytes, data_type);
    def.lsb = lsb;
    def.offset = offset;
    build_layout(vec![def], Vec::new()).value
}

// -------------------------------------------------------------- reported

#[test]
fn a_value_past_the_top_of_the_width_is_reported_with_what_was_written() {
    let layout = layout(1, DataType::UI, 1.0, 0.0);
    let encoded = layout.encode(&[(1, Value::Physical(300.0))]);

    assert_eq!(encoded.value, vec![255], "the clamp still happens");
    let issue = encoded
        .issues
        .iter()
        .find(|i| i.code == IssueCode::EncodeValueClamped)
        .unwrap_or_else(|| panic!("expected encode_value_clamped, got {:?}", encoded.issues));
    assert_eq!(issue.channel, Some(1));
    assert_eq!(issue.found.as_deref(), Some("300"));
    assert_eq!(issue.used.as_deref(), Some("255"), "what reached the wire");
}

#[test]
fn a_value_past_the_bottom_of_the_width_is_reported_too() {
    let layout = layout(1, DataType::UI, 1.0, 0.0);
    let encoded = layout.encode(&[(1, Value::Physical(-3.0))]);

    assert_eq!(encoded.value, vec![0]);
    let issue = encoded
        .issues
        .iter()
        .find(|i| i.code == IssueCode::EncodeValueClamped)
        .expect("a value below the width clamps too");
    assert_eq!(issue.found.as_deref(), Some("-3"));
    assert_eq!(issue.used.as_deref(), Some("0"));
}

#[test]
fn a_signed_channel_reports_both_of_its_ends() {
    let layout = layout(1, DataType::SI, 1.0, 0.0);

    for (given, written) in [(200.0, "127"), (-200.0, "-128")] {
        let encoded = layout.encode(&[(1, Value::Physical(given))]);
        let issue = encoded
            .issues
            .iter()
            .find(|i| i.code == IssueCode::EncodeValueClamped)
            .unwrap_or_else(|| panic!("{given} should clamp"));
        assert_eq!(issue.used.as_deref(), Some(written), "{given}");
    }
}

#[test]
fn the_reported_value_is_in_the_units_the_caller_used() {
    // lsb 0.5, offset -40: raw 255 is 87.5 degrees, not 255.
    let layout = layout(1, DataType::UI, 0.5, -40.0);
    let encoded = layout.encode(&[(1, Value::Physical(1000.0))]);

    let issue = encoded
        .issues
        .iter()
        .find(|i| i.code == IssueCode::EncodeValueClamped)
        .expect("1000 does not fit one byte at lsb 0.5");
    assert_eq!(issue.found.as_deref(), Some("1000"));
    assert_eq!(issue.used.as_deref(), Some("87.5"));
}

// ------------------------------------------------------------- not noise

#[test]
fn a_value_that_fits_reports_nothing() {
    let layout = layout(2, DataType::UI, 1.0, 0.0);

    for value in [0.0, 1.0, 65535.0] {
        let encoded = layout.encode(&[(1, Value::Physical(value))]);
        assert!(encoded.issues.is_empty(), "{value}: {:?}", encoded.issues);
    }
}

#[test]
fn a_value_that_only_rounds_is_not_a_clamp() {
    // §2 rounds half away from zero; losing a fraction is not the same as
    // losing the number.
    let layout = layout(1, DataType::UI, 1.0, 0.0);
    let encoded = layout.encode(&[(1, Value::Physical(2.5))]);

    assert_eq!(encoded.value, vec![3]);
    assert!(encoded.issues.is_empty(), "{:?}", encoded.issues);
}

#[test]
fn a_value_that_cannot_be_converted_is_still_the_other_issue() {
    let layout = layout(1, DataType::UI, 1.0, 0.0);
    let encoded = layout.encode(&[(1, Value::Physical(f64::NAN))]);

    let codes: Vec<IssueCode> = encoded.issues.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::EncodeValueInvalid], "not both");
}

#[test]
fn a_raw_value_past_the_width_is_still_the_raw_issue() {
    // §3 keeps the low bits and says so; that is a different mistake from
    // asking for a physical value the width cannot hold.
    let layout = layout(1, DataType::UI, 1.0, 0.0);
    let encoded = layout.encode(&[(1, Value::Raw(0x1FF))]);

    let codes: Vec<IssueCode> = encoded.issues.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::RawOutOfRange]);
}

// ------------------------------------------------- the primitive answers

#[test]
fn the_primitive_clamps_in_silence_and_can_be_asked_instead() {
    // §2: value_to_raw answers with a number, not with findings.
    let mut def = ChannelDef::new(1, 1, DataType::UI);
    def.lsb = 1.0;

    assert_eq!(def.value_to_raw(300.0), Some(255), "still clamps");
    assert!(!def.fits_width(300.0));
    assert!(!def.fits_width(-3.0));
    assert!(def.fits_width(255.0));
    assert!(def.fits_width(0.0));
}

#[test]
fn a_value_that_cannot_be_converted_does_not_fit_either() {
    let def = ChannelDef::new(1, 1, DataType::UI);
    assert!(!def.fits_width(f64::NAN));
    assert!(!def.fits_width(f64::INFINITY));
}

#[test]
fn the_ask_follows_the_channel_terms_rather_than_the_raw_width() {
    // lsb and offset move where the width lies in physical terms.
    let mut def = ChannelDef::new(1, 1, DataType::UI);
    def.lsb = 0.5;
    def.offset = -40.0;

    assert!(def.fits_width(-40.0), "raw 0");
    assert!(def.fits_width(87.5), "raw 255");
    assert!(!def.fits_width(88.0), "past raw 255");
    assert!(!def.fits_width(-40.5), "below raw 0");
}
