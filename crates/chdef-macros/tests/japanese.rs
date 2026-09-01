//! The options that change what is read: the Japanese column vocabulary,
//! and a recipe that reaches the table as the ranges it covers.

chdef_macros::layout!("tests/fixtures/ja.csv", japanese);

mod derived {
    chdef_macros::layout!("tests/fixtures/derived.csv");
}

#[test]
fn the_japanese_option_reads_the_japanese_header() {
    assert_eq!(LAYOUT.total, 2);
    assert_eq!(LAYOUT.slots[0].number, 1);
}

#[test]
fn a_recipe_becomes_the_bytes_it_covers() {
    assert_eq!(derived::LAYOUT.derived.len(), 1);
    assert_eq!(
        derived::LAYOUT.derived[0].covers,
        [chdef_core::Range { at: 0, len: 2 }]
    );
}

#[test]
fn a_sealed_frame_verifies_against_the_expanded_table() {
    let mut frame = [7u8, 0, 0, 0];

    assert!(
        derived::LAYOUT.seal(&mut frame),
        "the frame could not be sealed"
    );
    assert!(
        derived::LAYOUT.verify(&frame),
        "a sealed frame does not verify"
    );
}
