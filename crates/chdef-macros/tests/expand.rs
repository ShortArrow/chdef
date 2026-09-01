//! The macro expands to the table `chdef-gen` writes, and the constants it
//! declares answer as the golden vectors of the definition say
//! (`crates/chdef/vectors/basic/vectors.txt`).

chdef_macros::layout!(
    "../chdef/vectors/basic/ch.csv",
    bf = "../chdef/vectors/basic/bf.csv"
);

mod big {
    chdef_macros::layout!(
        "../chdef/vectors/basic/ch.csv",
        bf = "../chdef/vectors/basic/bf.csv",
        endian = big,
    );
}

/// A wire frame as the vectors spell it.
fn frame(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex is ASCII"), 16)
                .expect("a pair of hex digits")
        })
        .collect()
}

/// The same, the other way round.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn the_layout_is_the_one_the_l_line_states() {
    // L 13 1:0:4;2:4:2;3:6:1;4:7:2;5:9:4
    assert_eq!(LAYOUT.total, 13);

    let slots: Vec<(u32, u32, u8)> = LAYOUT
        .slots
        .iter()
        .map(|slot| (slot.number, slot.at, slot.bytes))
        .collect();
    assert_eq!(
        slots,
        vec![(1, 0, 4), (2, 4, 2), (3, 6, 1), (4, 7, 2), (5, 9, 4)]
    );
}

#[test]
fn a_named_channel_becomes_a_constant_carrying_its_number() {
    assert_eq!(CH_FRAME_COUNTER, 1);
    assert_eq!(CH_STATUS, 2);
}

#[test]
fn the_table_reads_the_raw_value_the_d_line_states() {
    // D 0100000005000285ffdc050000 … 4=65413/-12.3
    let frame = frame("0100000005000285ffdc050000");

    assert_eq!(LAYOUT.read(&frame, 4), Some(65413));
}

#[test]
fn the_defaults_are_the_frame_the_e_line_states() {
    // E - 00000000010000000000000000
    let mut frame = vec![0u8; 13];

    assert!(LAYOUT.fill_defaults(&mut frame), "the frame is long enough");
    assert_eq!(hex(&frame), "00000000010000000000000000");
}

#[test]
fn the_endian_option_reaches_the_table() {
    assert_eq!(big::LAYOUT.endian, chdef_core::Endian::Big);
    assert_eq!(big::LAYOUT.total, 13);
}

#[test]
fn a_definition_with_no_recipe_has_no_derived_channel() {
    assert!(LAYOUT.derived.is_empty());
}
