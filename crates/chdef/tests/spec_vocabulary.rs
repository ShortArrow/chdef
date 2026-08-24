//! Identifying columns (`docs/spec/format.md` §2): the canonical name is
//! the column identity, and every other spelling is a vocabulary the
//! caller supplies.
//!
//! ADR-0024. The point of each test here is that no language is built into
//! the mechanism, so a vocabulary chdef ships and one a caller writes
//! behave identically.

use chdef::*;

const JA_CH: &str = "番号,バイト数,メッセージ名称,型\n7,4,Frame,UI32\n";
const DE_CH: &str = "Nummer,Bytes,Bezeichnung,Typ\n7,4,Frame,UI32\n";

/// A vocabulary for a language chdef has never heard of, built the only
/// way any vocabulary is built.
fn german() -> ColumnVocabulary {
    ColumnVocabulary::new()
        .ch("Nummer", ChColumn::Number)
        .ch("Bytes", ChColumn::Bytes)
        .ch("Bezeichnung", ChColumn::Name)
        .ch("Typ", ChColumn::Type)
}

// ------------------------------------------------------- the identity

#[test]
fn a_column_is_named_by_its_canonical_name_alone() {
    // §2: the canonical name is the column identity, and no language
    // selects it.
    assert_eq!(ChColumn::Number.name(), "number");
    assert_eq!(ChColumn::Favorite.name(), "favorite");
    assert_eq!(BfColumn::Bit.name(), "bit");
}

#[test]
fn the_canonical_name_is_read_with_no_vocabulary_at_all() {
    let parsed = ChTable::parse("number,bytes,name,type\n7,4,Frame,UI32\n")
        .unwrap()
        .channels();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    assert_eq!(parsed.value[0].number, 7);
    assert_eq!(parsed.value[0].name, "Frame");
}

#[test]
fn the_variants_of_a_column_are_read_with_no_vocabulary_either() {
    // §3: the variants listed beside each canonical name are part of the
    // identity, not vocabulary.
    let parsed = ChTable::parse("no,bytes,SignalName\n7,4,Frame\n")
        .unwrap()
        .channels();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    assert_eq!(parsed.value[0].number, 7);
    assert_eq!(parsed.value[0].name, "Frame");
}

// --------------------------------------------- no language is built in

#[test]
fn a_language_chdef_never_heard_of_reads_exactly_like_one_it_ships() {
    let by_hand = ChTable::parse_with(DE_CH, &german()).unwrap().channels();
    let shipped = ChTable::parse_with(JA_CH, &ColumnVocabulary::japanese())
        .unwrap()
        .channels();

    assert!(by_hand.issues.is_empty(), "{:?}", by_hand.issues);
    assert!(shipped.issues.is_empty(), "{:?}", shipped.issues);
    assert_eq!(by_hand.value[0].number, shipped.value[0].number);
    assert_eq!(by_hand.value[0].name, shipped.value[0].name);
    assert_eq!(by_hand.value[0].byte_count, shipped.value[0].byte_count);
}

#[test]
fn a_japanese_header_is_not_read_until_its_vocabulary_is_asked_for() {
    // §2: reading with no vocabulary recognises the canonical names and
    // their variants alone. An unrecognised header falls back to position
    // and says so.
    let parsed = ChTable::parse(JA_CH).unwrap().channels();
    assert!(
        parsed
            .issues
            .iter()
            .any(|i| i.code == IssueCode::HeaderAssumed),
        "expected header_assumed, got {:?}",
        parsed.issues
    );
}

#[test]
fn the_canonical_name_stays_readable_under_any_vocabulary() {
    // §2: a vocabulary adds to the canonical names rather than replacing
    // them.
    let parsed = ChTable::parse_with("number,Bytes,name\n7,4,Frame\n", &german())
        .unwrap()
        .channels();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    assert_eq!(parsed.value[0].number, 7);
    assert_eq!(parsed.value[0].name, "Frame");
}

#[test]
fn a_vocabulary_cannot_reassign_a_canonical_name() {
    // §2: cells are matched against the canonical names first, so teaching
    // one to mean something else does nothing. The columns taught here are
    // chosen so that obeying the teaching would land every value in a
    // different field.
    let mischief = ColumnVocabulary::new()
        .ch("bytes", ChColumn::Name)
        .ch("name", ChColumn::Unit);

    let parsed = ChTable::parse_with(
        "number,bytes,name,unit
7,4,Frame,kPa
",
        &mischief,
    )
    .unwrap()
    .channels();

    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    let ch = &parsed.value[0];
    assert_eq!(ch.number, 7);
    assert_eq!(ch.byte_count, 4, "`bytes` is still the width");
    assert_eq!(ch.name, "Frame", "`name` is still the name");
    assert_eq!(ch.unit, "kPa");
}

#[test]
fn spellings_are_matched_trimmed_and_case_insensitively() {
    let parsed = ChTable::parse_with("  NUMMER , bytes \n7,4\n", &german())
        .unwrap()
        .channels();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    assert_eq!(parsed.value[0].number, 7);
}

// ------------------------------------------------------- the writer

#[test]
fn a_file_created_with_no_vocabulary_uses_the_canonical_names() {
    let csv = ChTable::new().to_csv();
    assert!(csv.contains("number,bytes,bits"), "{csv}");
}

#[test]
fn a_vocabulary_names_the_columns_of_a_file_it_creates() {
    // §2: a file chdef creates is spelled by the vocabulary the caller
    // asked for — the same for a shipped one and a hand-built one.
    let ja = ChTable::with_columns(
        &[ChColumn::Number, ChColumn::Bytes],
        &ColumnVocabulary::japanese(),
    )
    .to_csv();
    assert!(ja.contains("番号,バイト数"), "{ja}");

    let de = ChTable::with_columns(&[ChColumn::Number, ChColumn::Bytes], &german()).to_csv();
    assert!(de.contains("Nummer,Bytes"), "{de}");
}

#[test]
fn the_first_spelling_taught_for_a_column_is_the_one_written() {
    // §2, and the reason no separate setter exists.
    let vocabulary = ColumnVocabulary::new()
        .ch("Nummer", ChColumn::Number)
        .ch("Nr", ChColumn::Number);
    let csv = ChTable::with_columns(&[ChColumn::Number], &vocabulary).to_csv();
    assert!(csv.contains("Nummer"), "{csv}");
    assert!(!csv.contains("Nr,"), "{csv}");

    // Both still read.
    for header in ["Nummer\n7\n", "Nr\n7\n"] {
        let parsed = ChTable::parse_with(header, &vocabulary).unwrap().channels();
        assert_eq!(parsed.value[0].number, 7, "{header:?}");
    }
}

#[test]
fn a_column_the_vocabulary_does_not_name_is_written_canonically() {
    let csv = ChTable::with_columns(&[ChColumn::Number, ChColumn::Memo], &german()).to_csv();
    assert!(csv.contains("Nummer,memo"), "{csv}");
}

#[test]
fn a_file_a_vocabulary_writes_is_a_file_it_reads() {
    for vocabulary in [ColumnVocabulary::japanese(), german()] {
        let mut table = ChTable::with_columns(ChColumn::canonical(), &vocabulary);
        table.append_row(Vec::new());
        table.set_cell(0, 0, "7");
        table.set_cell(0, 1, "4");
        table.set_cell(0, 5, "UI32");

        let text = table.to_csv();
        let read = ChTable::parse_with(&text, &vocabulary).unwrap().channels();
        assert!(read.issues.is_empty(), "{:?}", read.issues);
        assert_eq!(read.value[0].number, 7);
        assert_eq!(read.value[0].byte_count, 4);
    }
}

// -------------------------------------------------------- composition

#[test]
fn vocabularies_compose() {
    let mixed = ColumnVocabulary::japanese().with(&german());
    for header in ["番号,バイト数\n7,4\n", "Nummer,Bytes\n7,4\n"] {
        let parsed = ChTable::parse_with(header, &mixed).unwrap().channels();
        assert!(parsed.issues.is_empty(), "{header:?}: {:?}", parsed.issues);
        assert_eq!(parsed.value[0].number, 7, "{header:?}");
    }
}

#[test]
fn a_bf_header_takes_a_vocabulary_the_same_way() {
    let vocabulary = ColumnVocabulary::new()
        .bf("Nummer", BfColumn::Number)
        .bf("Bit", BfColumn::Bit);
    let parsed = BfTable::parse_with("Nummer,Bit\n2,3\n", &vocabulary)
        .unwrap()
        .bitfields();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    assert_eq!(parsed.value[0].parent_channel, 2);
    assert_eq!(parsed.value[0].bit_number, 3);
}

#[test]
fn the_shipped_japanese_vocabulary_reads_every_column_it_prints() {
    // §2: the table the specification prints, read through the columns it
    // claims to name — a spelling on the wrong column shows up as a field
    // read from the wrong cell.
    let header = "番号,バイト数,ビット数,セクション名,メッセージ名称,型,\
                  LSB,オフセット,単位,値(最小),値(最大),値(デフォルト),\
                  備考,変数名,表示形式,お気に入り";
    let row = "7,4,,Body,Frame,UI32,1,-40,degC,0,100,0x10,note,frame_var,HEX,1";
    let source = format!("{header}\n{row}\n");

    let parsed = ChTable::parse_with(&source, &ColumnVocabulary::japanese())
        .unwrap()
        .channels();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);

    let ch = &parsed.value[0];
    assert_eq!(ch.number, 7);
    assert_eq!(ch.byte_count, 4);
    assert_eq!(ch.section, "Body");
    assert_eq!(ch.name, "Frame");
    assert_eq!(ch.lsb, 1.0);
    assert_eq!(ch.offset, -40.0);
    assert_eq!(ch.unit, "degC");
    assert_eq!(ch.min, Some(Value::Physical(0.0)));
    assert_eq!(ch.max, Some(Value::Physical(100.0)));
    assert_eq!(ch.default_value, Some(0x10));
    assert_eq!(ch.memo, "note");
    assert_eq!(ch.var, "frame_var");
    assert_eq!(ch.format, ValueDisplay::Raw);
    assert!(ch.favorite);
}

#[test]
fn the_shipped_japanese_vocabulary_reads_the_second_spellings_too() {
    // §2: the rows that carry more than one spelling.
    let header = "番号,信号名称,データ型,スケール,基準値,最小値,最大値,デフォルト値";
    let row = "7,Frame,UI16,0.5,-40,0,100,0x10";
    let source = format!("{header}\n{row}\n");

    let parsed = ChTable::parse_with(&source, &ColumnVocabulary::japanese())
        .unwrap()
        .channels();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);

    let ch = &parsed.value[0];
    assert_eq!(ch.name, "Frame");
    assert_eq!(ch.data_type, DataType::UI);
    assert_eq!(ch.lsb, 0.5);
    assert_eq!(ch.offset, -40.0);
    assert_eq!(ch.min, Some(Value::Physical(0.0)));
    assert_eq!(ch.max, Some(Value::Physical(100.0)));
    assert_eq!(ch.default_value, Some(0x10));
}

#[test]
fn the_shipped_japanese_vocabulary_reads_a_bf_header() {
    let source = "番号,BIT番号,メッセージ名称,値(デフォルト),備考\n2,3,alive,1,note\n";
    let parsed = BfTable::parse_with(source, &ColumnVocabulary::japanese())
        .unwrap()
        .bitfields();
    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);

    let bit = &parsed.value[0];
    assert_eq!(bit.parent_channel, 2);
    assert_eq!(bit.bit_number, 3);
    assert_eq!(bit.name, "alive");
    assert_eq!(bit.default_value, Some(1));
    assert_eq!(bit.memo, "note");
}
