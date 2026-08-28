//! What the two writers put on disk: source a firmware tree can keep
//! without exempting it from its own formatting, and a header whose tables
//! C will accept.

use std::path::{Path, PathBuf};
use std::process::Command;

use chdef::ColumnVocabulary;
use chdef_core::Endian;
use chdef_gen::{c_header, model, rust_source, Model};

fn vectors(set: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .join("chdef/vectors")
        .join(set)
}

fn basic(endian: Endian) -> Model {
    let directory = vectors("basic");
    let ch = std::fs::read(directory.join("ch.csv")).expect("ch.csv");
    let bf = std::fs::read(directory.join("bf.csv")).expect("bf.csv");
    model(&ch, &bf, endian, &ColumnVocabulary::new()).expect("the basic vectors load")
}

fn with_a_crc(plain: usize, recipes: &[&str]) -> Model {
    let mut csv = "number,bytes,type,kind,derived,name\n".to_string();
    for number in 1..=plain {
        csv.push_str(&format!("{number},4,UI,plain,,ch{number}\n"));
    }
    for (index, recipe) in recipes.iter().enumerate() {
        let number = plain + index + 1;
        csv.push_str(&format!("{number},2,UI,derived,{recipe},crc{number}\n"));
    }
    model(
        csv.as_bytes(),
        b"",
        Endian::Little,
        &ColumnVocabulary::new(),
    )
    .expect("the definition loads")
}

/// Whether `rustfmt` would leave the source exactly as it is.
fn is_rustfmt_clean(source: &str, name: &str) -> bool {
    let path = std::env::temp_dir().join(format!("chdef-gen-{name}-{}.rs", std::process::id()));
    std::fs::write(&path, source).expect("the sample could not be written");
    let status = Command::new("rustfmt")
        .args(["--check", "--edition", "2021"])
        .arg(&path)
        .status()
        .expect("rustfmt could not be run");
    let _ = std::fs::remove_file(&path);
    status.success()
}

#[test]
fn the_rust_table_is_written_as_rustfmt_would_leave_it() {
    let source = rust_source(&basic(Endian::Little), "ch.csv + bf.csv");

    assert!(
        is_rustfmt_clean(&source, "basic"),
        "rustfmt would rewrite:\n{source}"
    );
    assert!(source.contains("total: 13"), "{source}");
}

#[test]
fn a_table_with_derived_channels_is_written_the_same_way() {
    // The shapes rustfmt gives an array differ with its length, and the
    // covers of a derived channel sit one array inside another.
    for plain in 1..=6 {
        for count in 0..=2 {
            let span = format!("crc16/x25 1..{plain}");
            let recipes = vec![span.as_str(); count];
            let source = rust_source(&with_a_crc(plain, &recipes), "ch.csv");
            assert!(
                is_rustfmt_clean(&source, "derived"),
                "{plain} channels and {count} recipes; rustfmt would rewrite:\n{source}"
            );
        }
    }
}

#[test]
fn the_c_table_names_the_total_and_the_byte_order() {
    let header = c_header(&basic(Endian::Little), "ch.csv + bf.csv");

    assert!(header.contains("CHDEF_TOTAL_BYTES 13"), "{header}");
    assert!(header.contains("CHDEF_CORE_LITTLE"), "{header}");
}

#[test]
fn the_byte_order_asked_for_is_the_one_written() {
    let model = basic(Endian::Big);

    assert!(c_header(&model, "ch.csv").contains("CHDEF_CORE_BIG"));
    assert!(rust_source(&model, "ch.csv").contains("Endian::Big"));
}

#[test]
fn a_table_with_no_derived_channels_leaves_c_a_null_pointer() {
    // C has no empty initialiser list, so the absent table cannot be
    // declared empty.
    let header = c_header(&basic(Endian::Little), "ch.csv + bf.csv");

    assert!(header.contains("NULL, 0u"), "{header}");
    assert!(!header.contains("CHDEF_DERIVED"), "{header}");
}
