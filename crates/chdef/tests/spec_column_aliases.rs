//! Reading a file whose header uses spellings of the consumer's own
//! (`docs/spec/format.md` §2). An alias is a reader's convenience: it adds
//! a spelling for one reader, never changes what a canonical spelling
//! means, and never changes what is written.

use chdef::*;

// The spelling a consumer's own files use, taught to the reader.
#[test]
fn a_consumer_teaches_the_reader_its_own_spelling() {
    let aliases = ColumnAliases::new()
        .ch("signal id", ChColumn::Number)
        .ch("width", ChColumn::Bytes);

    let parsed = ChTable::parse_with("signal id,width,name\n7,4,Frame\n", &aliases)
        .unwrap()
        .channels();

    assert!(parsed.issues.is_empty(), "{:?}", parsed.issues);
    assert_eq!(parsed.value[0].number, 7);
    assert_eq!(parsed.value[0].byte_count, 4);
}

// Without the alias the same file has no `number` column, so it is read
// positionally — which is what makes the alias the thing that mattered.
#[test]
fn the_same_file_without_the_alias_is_read_positionally() {
    let parsed = ChTable::parse("signal id,width,name\n7,4,Frame\n")
        .unwrap()
        .channels();

    assert_eq!(parsed.issues[0].code, IssueCode::HeaderAssumed);
}

// An alias is added to the canonical spellings, not put in their place.
#[test]
fn the_canonical_spellings_keep_working_alongside_an_alias() {
    let aliases = ColumnAliases::new().ch("signal id", ChColumn::Number);

    let parsed = ChTable::parse_with("番号,bytes,name\n1,2,a\n", &aliases)
        .unwrap()
        .channels();

    assert_eq!(parsed.value[0].number, 1);
    assert_eq!(parsed.value[0].byte_count, 2);
    assert!(parsed.issues.is_empty());
}

// An alias cannot take a canonical spelling away from its column: the
// format is not the caller's to redefine, only to extend.
#[test]
fn an_alias_cannot_displace_a_canonical_spelling() {
    let confused = ColumnAliases::new()
        .ch("number", ChColumn::Bytes)
        .ch("bytes", ChColumn::Unit);

    let parsed = ChTable::parse_with("number,bytes\n5,3\n", &confused)
        .unwrap()
        .channels();

    assert_eq!(parsed.value[0].number, 5, "`number` still names the number");
    assert_eq!(
        parsed.value[0].byte_count, 3,
        "`bytes` still names the width"
    );
}

// §2: header cells are trimmed and matched case-insensitively. An alias is
// matched the way every other spelling is.
#[test]
fn an_alias_is_matched_the_way_every_spelling_is() {
    let aliases = ColumnAliases::new().ch("SignalId", ChColumn::Number);

    let parsed = ChTable::parse_with(" signalid ,name\n1,a\n", &aliases)
        .unwrap()
        .channels();

    assert_eq!(parsed.value[0].number, 1);
}

// The BF file has its own columns and its own aliases.
#[test]
fn the_bf_columns_take_aliases_too() {
    let aliases = ColumnAliases::new()
        .bf("parent", BfColumn::Number)
        .bf("position", BfColumn::Bit);

    let parsed = BfTable::parse_with("parent,position,name\n2,3,alive\n", &aliases)
        .unwrap()
        .bitfields();

    assert_eq!(
        (parsed.value[0].parent_channel, parsed.value[0].bit_number),
        (2, 3)
    );
    assert!(parsed.issues.is_empty());
}

// Bytes go through the same door.
#[test]
fn bytes_take_aliases_too() {
    let aliases = ColumnAliases::new().ch("signal id", ChColumn::Number);

    let table = ChTable::parse_bytes_with(b"signal id,name\n5,a\n", &aliases).unwrap();

    assert_eq!(table.channels().value[0].number, 5);
}

// An alias changes reading only. §2's write rule is unchanged: the header
// is written back exactly as it was read, so a file keeps its own wording
// rather than being canonicalised behind the consumer's back.
#[test]
fn an_alias_never_changes_what_is_written() {
    let aliases = ColumnAliases::new().ch("signal id", ChColumn::Number);
    let source = "signal id,name\n7,Frame\n";

    let table = ChTable::parse_with(source, &aliases).unwrap();

    assert_eq!(table.to_csv(), source);
    assert_eq!(table.header().map(|h| h[0].as_str()), Some("signal id"));
}

// And a file chdef creates never uses one: a new file is canonical.
#[test]
fn a_new_file_is_never_written_in_an_alias() {
    let aliases = ColumnAliases::new().ch("signal id", ChColumn::Number);
    let _ = &aliases;

    let table = ChTable::new();

    assert_eq!(table.header().map(|h| h[0].as_str()), Some("number"));
}

// Teaching the same spelling twice is the last word, not an error.
#[test]
fn the_last_alias_for_a_spelling_wins() {
    let aliases = ColumnAliases::new()
        .ch("id", ChColumn::Bytes)
        .ch("id", ChColumn::Number);

    let parsed = ChTable::parse_with("id,name\n4,a\n", &aliases)
        .unwrap()
        .channels();

    assert_eq!(parsed.value[0].number, 4);
}
