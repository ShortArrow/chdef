//! Who fills a channel (`docs/spec/format.md` §5): the `kind` column is a
//! mark chdef carries, and nothing chdef acts on.
//!
//! ADR-0025. Every test here exists because the fact used to live in a
//! constant somewhere else and went silently wrong when a row moved.

use chdef::*;

const CH: &str = "number,bytes,type,kind,default,name\n\
                  1,2,UI,const,0x7E7E,SYNC\n\
                  2,2,UI,counter,,FRAME_NO\n\
                  3,1,UI,,,PAYLOAD\n";

fn channels(source: &str) -> Parsed<Vec<ChannelDef>> {
    parse_ch_csv(source).unwrap()
}

// ------------------------------------------------------- reading the mark

#[test]
fn the_column_says_who_decides_each_channels_value() {
    let parsed = channels(CH);
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);

    let kinds: Vec<ChannelKind> = parsed.value.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        vec![ChannelKind::Const, ChannelKind::Counter, ChannelKind::Plain]
    );
}

#[test]
fn an_empty_cell_and_an_absent_column_both_mean_plain() {
    // §3: empty → plain. §2: a column absent from the header is
    // unspecified and is not an error.
    assert_eq!(channels(CH).value[2].kind, ChannelKind::Plain);

    let without = channels("number,bytes,type\n1,2,UI\n");
    assert!(without.issues.is_empty(), "{:?}", without.issues);
    assert_eq!(without.value[0].kind, ChannelKind::Plain);
}

#[test]
fn the_spellings_are_trimmed_and_case_insensitive() {
    let parsed = channels("number,bytes,kind\n1,2,  CONST \n2,2,Counter\n3,1, Plain \n");
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    assert_eq!(parsed.value[0].kind, ChannelKind::Const);
    assert_eq!(parsed.value[1].kind, ChannelKind::Counter);
    assert_eq!(
        parsed.value[2].kind,
        ChannelKind::Plain,
        "spelled, not merely absent"
    );
}

#[test]
fn a_value_this_chdef_does_not_know_is_plain_and_is_reported() {
    // §5: a file written for a later chdef still loads. `derived` stood
    // here until 0.0.11 gave it a meaning, which is the growth path
    // ADR-0025 described, arriving.
    let parsed = channels("number,bytes,kind\n1,2,computed\n");

    assert_eq!(parsed.value[0].kind, ChannelKind::Plain);
    let issue = parsed
        .issues
        .iter()
        .find(|i| i.code == IssueCode::KindAssumed)
        .unwrap_or_else(|| panic!("expected kind_assumed, got {:?}", parsed.issues));
    assert_eq!(issue.channel, Some(1));
    assert_eq!(issue.found.as_deref(), Some("computed"));
    assert_eq!(issue.used.as_deref(), Some("plain"));
}

// --------------------------------------------------- chdef does not act

#[test]
fn encode_produces_the_same_bytes_whatever_the_kind_says() {
    // §5, the whole point: kind changes no behaviour.
    let marked = build_layout(channels(CH).value, Vec::new()).value;

    let plain_source = "number,bytes,type,kind,default,name\n\
                        1,2,UI,,0x7E7E,SYNC\n\
                        2,2,UI,,,FRAME_NO\n\
                        3,1,UI,,,PAYLOAD\n";
    let plain = build_layout(channels(plain_source).value, Vec::new()).value;

    for values in [
        Vec::new(),
        vec![(2u32, Value::Raw(7))],
        vec![(1, Value::Raw(0xDEAD)), (3, Value::Physical(5.0))],
    ] {
        let a = marked.encode(&values);
        let b = plain.encode(&values);
        assert_eq!(a.value, b.value, "bytes differ for {values:?}");
        assert_eq!(
            a.issues.iter().map(|i| i.code).collect::<Vec<_>>(),
            b.issues.iter().map(|i| i.code).collect::<Vec<_>>(),
            "issues differ for {values:?}"
        );
    }
}

#[test]
fn overriding_a_const_channel_is_not_an_issue() {
    // ADR-0025: what a caller may send is the caller's to decide.
    let layout = build_layout(channels(CH).value, Vec::new()).value;
    let encoded = layout.encode(&[(1, Value::Raw(0xDEAD))]);

    assert!(encoded.issues.is_empty(), "{:?}", encoded.issues);
    assert_eq!(&encoded.value[..2], &[0xAD, 0xDE], "the value was written");
}

#[test]
fn a_counter_is_not_advanced_by_chdef() {
    // §5: a counter belongs to the line, so encoding twice with nothing
    // given yields the same frame both times.
    let layout = build_layout(channels(CH).value, Vec::new()).value;
    assert_eq!(layout.encode(&[]).value, layout.encode(&[]).value);
}

// ------------------------------------------------- carried and written back

#[test]
fn the_cell_survives_a_round_trip() {
    let table = ChTable::parse(CH).unwrap();
    assert_eq!(table.to_csv().trim_start_matches('\u{FEFF}'), CH);
}

#[test]
fn a_typed_insertion_writes_the_kind_into_the_column() {
    // editing.md §3: a typed definition is rendered into the columns this
    // file has.
    let mut table = ChTable::with_columns(
        &[ChColumn::Number, ChColumn::Bytes, ChColumn::Kind],
        &ColumnVocabulary::new(),
    );
    let mut def = ChannelDef::new(4, 2, DataType::UI);
    def.kind = ChannelKind::Counter;
    table.insert_channel(0, &def);

    assert_eq!(table.to_csv().lines().nth(1), Some("4,2,counter"));
    assert_eq!(table.channels().value[0].kind, ChannelKind::Counter);
}

#[test]
fn the_column_has_a_canonical_name_and_a_japanese_spelling() {
    assert_eq!(ChColumn::Kind.name(), "kind");
    assert_eq!(
        ColumnVocabulary::japanese().ch_spelling(ChColumn::Kind),
        "種別"
    );
}

#[test]
fn kind_is_appended_so_the_positional_form_is_untouched() {
    // ADR-0025: the first nine columns are frozen by the positional form.
    assert_eq!(ChColumn::canonical().len(), 18);
    assert!(ChColumn::canonical().contains(&ChColumn::Kind));
    assert_eq!(ChColumn::positional().len(), 9);
    assert!(!ChColumn::positional().contains(&ChColumn::Kind));
    assert!(
        !ChColumn::positional().contains(&ChColumn::Derived),
        "every column added since is appended too"
    );

    // A 9-column file still reads by position.
    let parsed = channels("1,4,,General,Frame,UI32,1,,\n");
    assert_eq!(parsed.value[0].number, 1);
    assert_eq!(parsed.value[0].byte_count, 4);
    assert_eq!(parsed.value[0].kind, ChannelKind::Plain);
}

#[test]
fn the_spelling_of_each_kind_is_what_the_column_holds() {
    assert_eq!(ChannelKind::Plain.as_str(), "plain");
    assert_eq!(ChannelKind::Const.as_str(), "const");
    assert_eq!(ChannelKind::Counter.as_str(), "counter");
    assert_eq!(ChannelKind::Derived.as_str(), "derived");
    assert_eq!(ChannelKind::default(), ChannelKind::Plain);
}
