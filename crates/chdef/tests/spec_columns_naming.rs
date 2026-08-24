//! Choosing the header of a file chdef creates
//! (`docs/spec/format.md` §2, ADR-0003: "For a new file the header language
//! is a parameter (`HeaderLanguage`); the default is English").

use chdef::*;

// §2: a new file uses the English spellings, in canonical order.
#[test]
fn a_new_table_uses_the_english_canonical_header() {
    let table = ChTable::new();

    assert_eq!(
        table.header().map(|h| h.join(",")),
        Some(
            "number,bytes,bits,section,name,type,lsb,offset,unit,min,max,default,memo,var,format,favorite"
                .to_string()
        )
    );
}

// ADR-0003: the header language of a new file is the caller's choice.
#[test]
fn a_new_table_can_be_created_in_japanese() {
    let table = ChTable::with_columns(ChColumn::positional(), HeaderLanguage::Ja);

    assert_eq!(
        table.header().map(|h| h.join(",")),
        Some(
            "番号,バイト数,ビット数,セクション名,メッセージ名称,型,LSB,オフセット,単位".to_string()
        )
    );
}

// A table with fewer columns writes fewer cells: a field with no column is
// dropped (`docs/spec/editing.md` §3).
#[test]
fn a_table_writes_only_the_columns_it_has() {
    let mut table = ChTable::with_columns(
        &[ChColumn::Number, ChColumn::Bytes, ChColumn::Name],
        HeaderLanguage::En,
    );
    let mut def = ChannelDef::new(1, 2, DataType::UI);
    def.name = "Status".into();
    def.unit = "kPa".into();

    table.insert_channel(0, &def);

    assert_eq!(table.to_csv().lines().nth(1), Some("1,2,Status"));
    assert!(table.channels().value[0].unit.is_empty());
}

// The BF table is created the same way.
#[test]
fn a_bf_table_chooses_its_header_too() {
    let table = BfTable::with_columns(BfColumn::canonical(), HeaderLanguage::Ja);

    assert_eq!(
        table.header().map(|h| h.join(",")),
        Some("番号,BIT番号,メッセージ名称,値(デフォルト),備考".to_string())
    );
}

// The canonical order and the positional fallback are readable, so a
// consumer can build its own header from them.
#[test]
fn the_canonical_orders_are_readable() {
    assert_eq!(ChColumn::canonical().len(), 16);
    assert_eq!(ChColumn::positional().len(), 9);
    assert_eq!(ChColumn::canonical()[0], ChColumn::Number);
    assert_eq!(BfColumn::canonical().len(), 5);

    assert_eq!(ChColumn::Number.name(HeaderLanguage::En), "number");
    assert_eq!(ChColumn::Number.name(HeaderLanguage::Ja), "番号");
    assert_eq!(BfColumn::Bit.name(HeaderLanguage::Ja), "BIT番号");
}

// §2: which column a header cell denotes, for an editor labelling a grid.
#[test]
fn a_header_cell_says_which_column_it_is() {
    assert_eq!(ChColumn::from_header("番号"), Some(ChColumn::Number));
    assert_eq!(ChColumn::from_header(" Default "), Some(ChColumn::Default));
    assert_eq!(ChColumn::from_header("unknown"), None);
    assert_eq!(BfColumn::from_header("BIT番号"), Some(BfColumn::Bit));
}

// A chosen header is read back as the header it is.
#[test]
fn a_chosen_header_survives_the_round_trip() {
    let mut table = ChTable::with_columns(
        &[ChColumn::Number, ChColumn::Name, ChColumn::Unit],
        HeaderLanguage::Ja,
    );
    let mut def = ChannelDef::new(3, 2, DataType::UI);
    def.name = "圧力".into();
    def.unit = "kPa".into();
    table.insert_channel(0, &def);

    let written = table.to_csv();
    let read_back = ChTable::parse(written.trim_start_matches('\u{FEFF}')).unwrap();

    assert_eq!(
        read_back.header().map(|h| h.join(",")),
        Some("番号,メッセージ名称,単位".to_string())
    );
    let channels = read_back.channels();
    assert!(channels.issues.is_empty(), "{:?}", channels.issues);
    assert_eq!(channels.value[0].number, 3);
    assert_eq!(channels.value[0].unit, "kPa");
}
