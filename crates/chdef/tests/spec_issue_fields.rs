//! An Issue has to be readable without reading English: a consumer writing
//! its own sentences needs the value chdef could not use, the value it used
//! instead, and which channel or bit the finding is about, as fields.

use chdef::*;

fn first(text: &str) -> Issue {
    parse_ch_csv(text)
        .unwrap()
        .issues
        .into_iter()
        .next()
        .unwrap()
}

// The value the file held, as the file spells it — the one thing a
// consumer cannot recover once parsing has replaced it.
#[test]
fn an_unreadable_cell_comes_back_as_it_was_written() {
    let issue = first("number,name\nnot a number,a\n");

    assert_eq!(issue.code, IssueCode::ChannelNumberInvalid);
    assert_eq!(issue.found.as_deref(), Some("not a number"));
}

// And the value chdef put in its place, where it substituted one.
#[test]
fn a_substituted_value_is_a_field_too() {
    let clamped = first("number,bytes,name\n1,99,a\n");
    assert_eq!(clamped.code, IssueCode::BytesOutOfRange);
    assert_eq!(clamped.found.as_deref(), Some("99"));
    assert_eq!(clamped.used.as_deref(), Some("8"));

    let assumed = first("number,type,name\n1,ZZ,a\n");
    assert_eq!(assumed.code, IssueCode::TypeAssumed);
    assert_eq!(assumed.found.as_deref(), Some("ZZ"));
    assert_eq!(assumed.used.as_deref(), Some("UI"));

    let lsb = first("number,lsb,name\n1,abc,a\n");
    assert_eq!(lsb.code, IssueCode::LsbInvalid);
    assert_eq!(lsb.found.as_deref(), Some("abc"));
    assert_eq!(lsb.used.as_deref(), Some("1"));
}

// A raw value keeps the notation its cell used, so the sentence a consumer
// builds reads like the file.
#[test]
fn a_raw_value_keeps_the_notation_of_its_cell() {
    let hex = first("number,bytes,type,default\n1,1,UI,0x1FF\n");
    assert_eq!(hex.code, IssueCode::RawOutOfRange);
    assert_eq!(hex.found.as_deref(), Some("0x1FF"));
    assert_eq!(hex.used.as_deref(), Some("0xFF"));

    let decimal = first("number,bytes,type,default\n1,1,UI,511\n");
    assert_eq!(decimal.found.as_deref(), Some("511"));
    assert_eq!(decimal.used.as_deref(), Some("255"));
}

// Which channel a finding is about, so a consumer names it without reading
// the sentence.
#[test]
fn a_finding_names_the_channel_it_is_about() {
    let issue = first("number,name\n1,a\n1,again\n");

    assert_eq!(issue.code, IssueCode::ChannelDuplicate);
    assert_eq!(issue.channel, Some(1));
    assert_eq!(issue.bit, None);
}

// A bit-field finding names both halves of its identity.
#[test]
fn a_bit_finding_names_the_channel_and_the_bit() {
    let parsed = parse_bf_csv("number,bit,name\n2,3,a\n2,3,again\n").unwrap();

    let issue = &parsed.issues[0];
    assert_eq!(issue.code, IssueCode::BfDuplicate);
    assert_eq!((issue.channel, issue.bit), (Some(2), Some(3)));
}

// The cross-file findings carry no row (ADR-0008), so the identity fields
// are the only way to say which row a consumer should look at.
#[test]
fn a_rowless_finding_still_names_what_it_is_about() {
    let channels = vec![ChannelDef::new(1, 2, DataType::UI)];
    let bitfields = vec![BitFieldDef::new(1, 0)];

    let built = build_layout(channels, bitfields);

    let issue = &built.issues[0];
    assert_eq!(issue.code, IssueCode::BfParentNotBitfield);
    assert_eq!(issue.row, None);
    assert_eq!((issue.channel, issue.bit), (Some(1), Some(0)));
}

// Encoding reports against a channel, not a row.
#[test]
fn an_encode_finding_names_its_channel() {
    let layout = build_layout(vec![ChannelDef::new(1, 2, DataType::UI)], vec![]).value;

    let issues = layout.encode(&[(9, Value::Raw(1))]).issues;

    assert_eq!(issues[0].code, IssueCode::EncodeUnknownChannel);
    assert_eq!(issues[0].channel, Some(9));
}

// A finding with nothing to report in a field leaves it empty rather than
// inventing something.
#[test]
fn a_finding_with_nothing_to_report_leaves_the_fields_empty() {
    let issue = first("1,2,,Sec,Name,UI,1,,\n");

    assert_eq!(issue.code, IssueCode::HeaderAssumed);
    assert_eq!(issue.channel, None);
    assert_eq!(issue.bit, None);
    assert_eq!(issue.found, None);
    assert_eq!(issue.used, None);
}

// The English sentence stays, for a log and for a reader who wants one.
#[test]
fn the_english_sentence_is_still_there() {
    let issue = first("number,name\nx,a\n");

    assert!(issue.message.contains("number"));
}

// The JSON carries the fields, so a consumer across the wire translates
// without the sentence too.
#[cfg(feature = "serde")]
#[test]
fn the_json_carries_the_fields() {
    let parsed = parse_ch_csv("number,bytes,name\n1,99,a\n").unwrap();
    let layout = build_layout(parsed.value, vec![]).value;

    let json =
        serde_json::to_value(chdef::interchange::Definitions::of(&layout, &parsed.issues)).unwrap();

    let issue = &json["issues"][0];
    assert_eq!(issue["code"], "bytes_out_of_range");
    assert_eq!(issue["found"], "99");
    assert_eq!(issue["used"], "8");
    assert_eq!(issue["channel"], 1);
}
