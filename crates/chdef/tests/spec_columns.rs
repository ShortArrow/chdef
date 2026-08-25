//! Tests derived from the wording of `docs/spec/format.md` §3 / §4 and
//! `docs/spec/editing.md`, on the columns whose handling the audit found
//! disagreeing with the specification.

use chdef::*;

// format.md §3, `default`: "`0x` / `0X` prefix → hexadecimal raw value.
// Anything else → decimal **raw value** (integer)." A raw value is the
// channel's whole width — conversion.md §4 calls it "a raw value" with no
// 32-bit ceiling anywhere in sight.
#[test]
fn a_default_may_be_as_wide_as_the_channel() {
    let parsed = parse_ch_csv("number,bytes,type,default\n1,8,UI,0x0102030405060708\n").unwrap();

    assert!(
        parsed.issues.is_empty(),
        "a default inside an 8-byte channel is not a problem: {:?}",
        parsed.issues
    );
    let layout = build_layout(parsed.value, vec![]).value;
    assert_eq!(layout.channel_default(1), Some(0x0102030405060708));
}

// §3: "Non-integer → unspecified (Issue `default_invalid`)." A well-formed
// `0x` value that simply does not fit is out of range, not unreadable —
// the same verdict `min` / `max` get for the same text.
#[test]
fn a_default_past_the_width_is_out_of_range_not_invalid() {
    let parsed = parse_ch_csv("number,bytes,type,default\n1,1,UI,0x1FF\n").unwrap();

    let codes: Vec<IssueCode> = parsed.issues.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::RawOutOfRange]);
    assert_eq!(parsed.value[0].default_value, Some(0xFF));
}

// The same text in the same file must get the same verdict in `default`
// and in `min` — format.md §3 says `min` is "width-checked like `default`".
#[test]
fn default_and_min_judge_the_same_text_the_same_way() {
    let parsed =
        parse_ch_csv("number,bytes,type,default,min\n1,4,UI,0x1FFFFFFFF,0x1FFFFFFFF\n").unwrap();

    let codes: Vec<IssueCode> = parsed.issues.iter().map(|i| i.code).collect();
    assert_eq!(
        codes,
        vec![IssueCode::RawOutOfRange, IssueCode::RawOutOfRange]
    );
    assert_eq!(parsed.value[0].default_value, Some(0xFFFF_FFFF));
    assert_eq!(parsed.value[0].min, Some(Value::Raw(0xFFFF_FFFF)));
}

// §3 treats a decimal `default` as a raw value too, so the width applies to
// it just as it does to the `0x` form.
#[test]
fn a_decimal_default_is_width_checked_too() {
    let parsed = parse_ch_csv("number,bytes,type,default\n1,1,UI,511\n").unwrap();

    let codes: Vec<IssueCode> = parsed.issues.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::RawOutOfRange]);
    assert_eq!(parsed.value[0].default_value, Some(0xFF));
}

// format.md §4, BF `bit`: "Integer ≥ 0 and below the parent width.
// Non-integer → row skipped (Issue `bf_bit_invalid`). ≥ width → row skipped
// (Issue `bf_bit_out_of_range`)." 256 is an integer, and it is ≥ the width.
#[test]
fn a_bit_number_past_the_widest_channel_is_out_of_range_not_invalid() {
    let parsed = parse_bf_csv("number,bit,name\n2,256,too far\n").unwrap();

    let codes: Vec<IssueCode> = parsed.issues.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::BfBitOutOfRange]);
}

// §4 still calls a non-integer `bit` invalid.
#[test]
fn a_bit_number_that_is_not_an_integer_is_invalid() {
    let parsed = parse_bf_csv("number,bit,name\n2,y,not a number\n").unwrap();

    let codes: Vec<IssueCode> = parsed.issues.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::BfBitInvalid]);
}

// format.md §3, `favorite`: "Written as `1` / `0`."
#[test]
fn favorite_is_written_as_one_or_zero() {
    let mut table = ChTable::new();
    let plain = ChannelDef::new(1, 2, DataType::UI);
    let mut pinned = ChannelDef::new(2, 2, DataType::UI);
    pinned.favorite = true;

    table.insert_channel(0, &plain);
    table.insert_channel(1, &pinned);

    let csv = table.to_csv();
    let mut lines = csv.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let at = header
        .iter()
        .position(|c| c.trim_start_matches('\u{FEFF}') == "favorite")
        .expect("the canonical header names favorite");
    let rows: Vec<&str> = lines.collect();
    let cell = |row: &str| row.split(',').nth(at).unwrap_or("").to_string();

    assert_eq!(
        cell(rows[0]),
        "0",
        "a plain channel writes 0: {:?}",
        rows[0]
    );
    assert_eq!(
        cell(rows[1]),
        "1",
        "a pinned channel writes 1: {:?}",
        rows[1]
    );
}

// conversion.md §5: encode takes "a raw value (`0x`)" per channel, and §3
// says bits beyond the width are reported. A value the caller cannot place
// as given is never dropped in silence (diagnostics.md §1).
#[test]
fn encode_reports_a_raw_value_past_the_width() {
    let layout = build_layout(vec![ChannelDef::new(1, 1, DataType::UI)], vec![]).value;

    let encoded = layout.encode(&[(1, Value::Raw(0x1FF))]);

    assert_eq!(encoded.value, vec![0xFF]);
    let codes: Vec<IssueCode> = encoded.issues.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::RawOutOfRange]);
}

// editing.md §3 lists `remove_row` beside `insert_row`, which clamps, and
// `set_cell`, which ignores a row outside the grid. Removing one that is
// not there is the same kind of miss, not a crash.
#[test]
fn removing_a_row_that_is_not_there_is_not_a_crash() {
    let mut table = ChTable::parse("number,name\n1,a\n").unwrap();

    assert_eq!(table.remove_row(5), None);
    assert_eq!(
        table.remove_row(0),
        Some(vec!["1".to_string(), "a".to_string()])
    );
    assert_eq!(table.row_count(), 0);
}

// editing.md §5: `Renumbered` "lists each `(old, new)` pair". A number that
// appears on several rows is one channel, so it moved once.
#[test]
fn renumbering_lists_each_channel_once() {
    let mut table = ChTable::parse("number,name\n3,a\n3,duplicate\n4,b\n").unwrap();

    let report = table.insert_channel_renumbering(0, &ChannelDef::new(3, 2, DataType::UI), None);

    assert_eq!(report.moved, vec![(3, 4), (4, 5)]);
}

// format.md §3: `number` has "No upper bound (u32)", so the largest one is
// legal input and renumbering past it must not wrap or crash.
#[test]
fn renumbering_past_the_largest_number_is_reported_not_wrapped() {
    let mut table = ChTable::parse(&format!("number,name\n{},a\n", u32::MAX)).unwrap();

    let report = table.insert_channel_renumbering(0, &ChannelDef::new(1, 2, DataType::UI), None);

    assert!(report.moved.is_empty(), "nothing can move past u32::MAX");
    assert_eq!(table.cell(1, 0), Some(u32::MAX.to_string().as_str()));
}
