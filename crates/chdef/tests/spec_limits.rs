//! The limits a layout is measured against (`docs/spec/layout.md` §5):
//! the byte capacity of ADR-0016, and the channel count beside it.
//!
//! Both are stated by the consumer, never inferred, and never applied by a
//! conversion — `check_capacity` is the explicit ask.

use chdef::*;

fn layout_of(channels: usize) -> ChannelLayout {
    let defs = (1..=channels as u32)
        .map(|n| ChannelDef::new(n, 2, DataType::UI))
        .collect();
    build_layout(defs, Vec::new()).value
}

#[test]
fn a_layout_with_no_limit_stated_reports_nothing() {
    assert!(layout_of(300).check_capacity().is_empty());
}

#[test]
fn a_frame_that_fits_both_limits_reports_nothing() {
    let layout = layout_of(10).with_capacity(246).with_channel_capacity(64);
    assert!(layout.check_capacity().is_empty(), "20 bytes, 10 channels");
}

#[test]
fn a_frame_longer_than_its_byte_capacity_is_reported() {
    let layout = layout_of(10).with_capacity(8);
    let issues = layout.check_capacity();

    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, IssueCode::LayoutExceedsCapacity);
    assert_eq!(issues[0].found.as_deref(), Some("20"));
    assert_eq!(issues[0].used.as_deref(), Some("8"));
}

#[test]
fn more_channels_than_the_port_accepts_is_reported() {
    // The limit a byte count cannot express: a 64-channel port takes 300
    // two-byte channels without complaint until the count is stated.
    let layout = layout_of(300).with_channel_capacity(64);
    let issues = layout.check_capacity();

    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, IssueCode::LayoutExceedsChannelCapacity);
    assert_eq!(issues[0].found.as_deref(), Some("300"));
    assert_eq!(issues[0].used.as_deref(), Some("64"));
}

#[test]
fn both_limits_are_reported_together_rather_than_one_of_them() {
    // The reason check_capacity answers with a list: a layout can be over
    // both, and a consumer that learns one at a time fixes one at a time.
    let layout = layout_of(300).with_capacity(8).with_channel_capacity(64);
    let codes: Vec<IssueCode> = layout.check_capacity().iter().map(|i| i.code).collect();

    assert_eq!(
        codes,
        vec![
            IssueCode::LayoutExceedsCapacity,
            IssueCode::LayoutExceedsChannelCapacity
        ]
    );
}

#[test]
fn a_limit_is_never_applied_on_its_own() {
    // layout.md §5: nothing calls this but the consumer.
    let layout = layout_of(300).with_capacity(8).with_channel_capacity(64);

    let encoded = layout.encode(&[]);
    assert_eq!(encoded.value.len(), 600, "encode ignores both limits");
    assert!(encoded.issues.is_empty(), "{:?}", encoded.issues);
    assert_eq!(layout.decode(&encoded.value).len(), 300);
}

#[test]
fn the_limits_are_readable_after_being_stated() {
    let layout = layout_of(4).with_capacity(246).with_channel_capacity(64);
    assert_eq!(layout.capacity, Some(246));
    assert_eq!(layout.channel_capacity, Some(64));
}
