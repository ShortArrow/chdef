//! Choosing the header of a file chdef creates
//! (`docs/spec/format.md` §2, ADR-0024: the canonical name is the column
//! identity and the spelling written comes from a vocabulary).
//!
//! Which spellings a vocabulary accepts is `spec_vocabulary.rs`; this file
//! is about the header a caller ends up with.

use chdef::*;

// §2: the default is every column in canonical order under the canonical
// names.
#[test]
fn a_new_table_uses_the_canonical_header() {
    let table = ChTable::new();

    assert_eq!(
        table.header().map(|h| h.join(",")),
        Some(
            "number,bytes,bits,section,name,type,lsb,offset,unit,min,max,default,memo,var,format,favorite,kind"
                .to_string()
        )
    );
}

#[test]
fn a_new_bf_table_uses_the_canonical_header() {
    assert_eq!(
        BfTable::new().header().map(|h| h.join(",")),
        Some("number,bit,name,default,memo".to_string())
    );
}

// A table with fewer columns writes fewer cells: a field with no column is
// dropped (`docs/spec/editing.md` §3).
#[test]
fn a_table_writes_only_the_columns_it_has() {
    let mut table = ChTable::with_columns(
        &[ChColumn::Number, ChColumn::Bytes, ChColumn::Name],
        &ColumnVocabulary::new(),
    );
    let mut def = ChannelDef::new(1, 2, DataType::UI);
    def.name = "Status".into();
    def.unit = "kPa".into();

    table.insert_channel(0, &def);

    assert_eq!(table.to_csv().lines().nth(1), Some("1,2,Status"));
    assert!(table.channels().value[0].unit.is_empty());
}

// The canonical order and the positional fallback are readable, so a
// consumer can build its own header from them.
#[test]
fn the_canonical_orders_are_readable() {
    assert_eq!(ChColumn::canonical().len(), 17);
    assert_eq!(ChColumn::positional().len(), 9);
    assert_eq!(ChColumn::canonical()[0], ChColumn::Number);
    assert_eq!(BfColumn::canonical().len(), 5);
}

// §2: which column a header cell denotes, for an editor labelling a grid.
#[test]
fn a_header_cell_says_which_column_it_is() {
    assert_eq!(ChColumn::from_header("number"), Some(ChColumn::Number));
    assert_eq!(ChColumn::from_header(" Default "), Some(ChColumn::Default));
    assert_eq!(ChColumn::from_header("unknown"), None);
    assert_eq!(BfColumn::from_header("BitNumber"), Some(BfColumn::Bit));
}

// A chosen header is read back as the header it is — by the vocabulary
// that wrote it.
#[test]
fn a_chosen_header_survives_the_round_trip() {
    let japanese = ColumnVocabulary::japanese();
    let mut table = ChTable::with_columns(
        &[ChColumn::Number, ChColumn::Name, ChColumn::Unit],
        &japanese,
    );
    let mut def = ChannelDef::new(3, 2, DataType::UI);
    def.name = "圧力".into();
    def.unit = "kPa".into();
    table.insert_channel(0, &def);

    let written = table.to_csv();
    let read_back = ChTable::parse_with(written.trim_start_matches('\u{FEFF}'), &japanese).unwrap();

    assert_eq!(
        read_back.header().map(|h| h.join(",")),
        Some("番号,メッセージ名称,単位".to_string())
    );
    let channels = read_back.channels();
    assert!(channels.issues.is_empty(), "{:?}", channels.issues);
    assert_eq!(channels.value[0].number, 3);
    assert_eq!(channels.value[0].unit, "kPa");
}

// The same file read without that vocabulary is not read as a header at
// all — the behaviour ADR-0024 chose deliberately.
#[test]
fn the_same_header_without_its_vocabulary_falls_back_to_position() {
    let japanese = ColumnVocabulary::japanese();
    let table = ChTable::with_columns(ChColumn::positional(), &japanese);

    let parsed = ChTable::parse(&table.to_csv()).unwrap().channels();
    assert!(
        parsed
            .issues
            .iter()
            .any(|i| i.code == IssueCode::HeaderAssumed),
        "{:?}",
        parsed.issues
    );
}
