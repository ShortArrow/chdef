//! Asking whether a value is inside its channel's declared range
//! (`docs/spec/conversion.md` §8).
//!
//! Three asks, because there are three places a value can be: about to be
//! sent, just received, and sitting in a cell of the definition. None of
//! them changes what `encode` or `decode` do, and none is remembered — a
//! range is a question of the moment, not a mode.

use chdef::*;

/// Channel 1 is 0..100, channel 2 is unbounded, channel 3 has only a floor,
/// and channel 4 is 0..100 at an `lsb` that makes raw and physical differ.
const CH: &str = "number,bytes,type,lsb,min,max,default\n\
                  1,2,UI,1,0,100,50\n\
                  2,2,UI,1,,,7\n\
                  3,2,SI,1,-10,,0\n\
                  4,2,UI,0.5,0,100,0\n";

fn layout() -> ChannelLayout {
    let parsed = parse_ch_csv(CH).unwrap();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    build_layout(parsed.value, Vec::new()).value
}

// -------------------------------------------------- before it is sent

#[test]
fn a_value_outside_its_range_is_named_with_the_bound_it_crossed() {
    let issues = layout().values_out_of_range(&[(1, Value::Physical(150.0))]);

    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, IssueCode::ValueOutOfRange);
    assert_eq!(issues[0].channel, Some(1));
    assert_eq!(issues[0].found.as_deref(), Some("150"));
    assert_eq!(issues[0].used.as_deref(), Some("100"), "the bound crossed");
}

#[test]
fn the_bound_named_is_the_side_that_was_crossed() {
    let issues = layout().values_out_of_range(&[(1, Value::Physical(-1.0))]);
    assert_eq!(
        issues[0].used.as_deref(),
        Some("0"),
        "the floor, not the top"
    );
}

#[test]
fn a_value_inside_its_range_is_not_named() {
    let issues = layout().values_out_of_range(&[
        (1, Value::Physical(0.0)),
        (1, Value::Physical(100.0)),
        (1, Value::Physical(50.0)),
    ]);
    assert!(issues.is_empty(), "the bounds are inclusive: {issues:?}");
}

#[test]
fn an_unbounded_side_bounds_nothing() {
    let issues = layout().values_out_of_range(&[
        (2, Value::Physical(1e9)),
        (3, Value::Physical(1e9)),
        (3, Value::Physical(-9.0)),
    ]);
    assert!(issues.is_empty(), "{issues:?}");

    let below = layout().values_out_of_range(&[(3, Value::Physical(-11.0))]);
    assert_eq!(below.len(), 1, "the floor still bounds");
    assert_eq!(below[0].used.as_deref(), Some("-10"));
}

#[test]
fn a_raw_value_is_judged_by_what_it_means() {
    // §8 resolves a bound with the channel's current lsb / offset, so a
    // raw value has to be read the same way before it is compared. Channel
    // 4 has lsb 0.5, so the two readings of the same bits disagree.
    let out = layout().values_out_of_range(&[(4, Value::Raw(250))]);
    assert_eq!(out.len(), 1, "raw 250 means 125, above 100: {out:?}");
    assert_eq!(out[0].found.as_deref(), Some("125"), "not the raw bits");

    let inside = layout().values_out_of_range(&[(4, Value::Raw(150))]);
    assert!(
        inside.is_empty(),
        "raw 150 means 75, inside 0..100: {inside:?}"
    );
}

#[test]
fn a_value_for_a_channel_the_layout_does_not_have_is_not_a_range_finding() {
    let issues = layout().values_out_of_range(&[(9, Value::Physical(1e9))]);
    assert!(issues.is_empty(), "encode is what reports that: {issues:?}");
}

#[test]
fn asking_changes_nothing_about_what_is_written() {
    let layout = layout();
    let values = [(1, Value::Physical(150.0))];

    let before = layout.encode(&values);
    let issues = layout.values_out_of_range(&values);
    let after = layout.encode(&values);

    assert!(!issues.is_empty(), "the ask found something");
    assert_eq!(before.value, after.value, "the frame is untouched");
    assert!(before.issues.is_empty(), "encode still says nothing");
    assert!(after.issues.is_empty());
}

// --------------------------------------------------- after it arrives

#[test]
fn a_reading_outside_its_range_is_named() {
    let layout = layout();
    let frame = layout.encode(&[(1, Value::Physical(150.0))]).value;

    let issues = layout.readings_out_of_range(&layout.decode(&frame));

    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, IssueCode::ValueOutOfRange);
    assert_eq!(issues[0].channel, Some(1));
    assert_eq!(issues[0].used.as_deref(), Some("100"));
}

#[test]
fn each_reading_is_judged_against_its_own_channel() {
    // Channel 2 is unbounded and channel 3 has only a floor; judging
    // either by another channel's range would answer differently.
    let layout = layout();
    let frame = layout
        .encode(&[(2, Value::Physical(60000.0)), (3, Value::Physical(-100.0))])
        .value;

    let issues = layout.readings_out_of_range(&layout.decode(&frame));

    assert_eq!(issues.len(), 1, "only channel 3 is out: {issues:?}");
    assert_eq!(issues[0].channel, Some(3));
    assert_eq!(issues[0].found.as_deref(), Some("-100"));
    assert_eq!(issues[0].used.as_deref(), Some("-10"));
}

#[test]
fn a_frame_whose_readings_all_fit_is_quiet() {
    let layout = layout();
    let frame = layout.encode(&[]).value;
    assert!(
        layout
            .readings_out_of_range(&layout.decode(&frame))
            .is_empty(),
        "the defaults are inside their ranges"
    );
}

// -------------------------------------------- sitting in a cell

#[test]
fn a_default_outside_its_own_row_range_points_at_the_cell() {
    // §8: the finding carries the grid row and the `default` column, so an
    // editor colours the cell.
    let table = ChTable::parse(
        "number,bytes,type,lsb,min,max,default\n\
         1,2,UI,1,0,100,50\n\
         2,2,UI,1,0,100,150\n",
    )
    .unwrap();

    let issues = table.defaults_out_of_range();

    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, IssueCode::ValueOutOfRange);
    assert_eq!(issues[0].row, Some(1), "the second data row");
    assert_eq!(
        issues[0].col,
        table
            .header()
            .and_then(|h| h.iter().position(|c| c == "default")),
        "the default column"
    );
    assert_eq!(issues[0].channel, Some(2));
    assert_eq!(issues[0].found.as_deref(), Some("150"));
    assert_eq!(issues[0].used.as_deref(), Some("100"));
}

#[test]
fn a_row_with_no_default_or_no_range_is_not_a_finding() {
    let table = ChTable::parse(
        "number,bytes,type,lsb,min,max,default\n\
         1,2,UI,1,0,100,\n\
         2,2,UI,1,,,150\n\
         3,2,UI,1,0,100,100\n",
    )
    .unwrap();
    assert!(
        table.defaults_out_of_range().is_empty(),
        "{:?}",
        table.defaults_out_of_range()
    );
}

#[test]
fn a_file_without_a_default_column_has_nothing_to_point_at() {
    let table = ChTable::parse("number,bytes,min,max\n1,2,0,100\n").unwrap();
    assert!(table.defaults_out_of_range().is_empty());
}

#[test]
fn the_cell_finding_speaks_in_physical_terms_on_both_sides() {
    // `default` is raw and the bounds are physical, so `found` and `used`
    // would be in different units if the cell text were reported verbatim.
    // The editor already has the cell; what it does not have is what the
    // cell means.
    let table = ChTable::parse(
        "number,bytes,type,lsb,min,max,default\n\
         1,2,UI,0.5,0,100,250\n",
    )
    .unwrap();

    let issues = table.defaults_out_of_range();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(
        issues[0].found.as_deref(),
        Some("125"),
        "raw 250 at lsb 0.5"
    );
    assert_eq!(issues[0].used.as_deref(), Some("100"));
}

#[test]
fn the_cell_is_judged_after_the_row_terms_resolve_it() {
    // `default` is a raw value; the range is physical. lsb 0.5 puts raw 150
    // at 75, which is inside 0..100 — reading the cell as physical would
    // wrongly call it out of range.
    let table = ChTable::parse(
        "number,bytes,type,lsb,min,max,default\n\
         1,2,UI,0.5,0,100,150\n",
    )
    .unwrap();
    assert!(
        table.defaults_out_of_range().is_empty(),
        "{:?}",
        table.defaults_out_of_range()
    );
}
