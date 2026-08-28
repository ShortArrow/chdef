//! The identifier each channel's constant takes. A definition is written
//! for people, and `Frame Count` is not a name either language accepts, so
//! the generator cuts one down without asking the author to.

use chdef::ColumnVocabulary;
use chdef_core::Endian;
use chdef_gen::{c_header, model, rust_source, Model};

/// A definition of `(name, var)` channels, numbered from 1.
fn named(channels: &[(&str, &str)]) -> Model {
    let mut csv = "number,bytes,type,name,var\n".to_string();
    for (index, (name, var)) in channels.iter().enumerate() {
        csv.push_str(&format!("{},2,UI,{name},{var}\n", index + 1));
    }
    model(
        csv.as_bytes(),
        b"",
        Endian::Little,
        &ColumnVocabulary::new(),
    )
    .expect("the definition loads")
}

#[test]
fn a_name_becomes_an_identifier() {
    assert_eq!(
        named(&[("Frame Count", "")]).names,
        vec![(1, "FRAME_COUNT".to_string())]
    );
}

#[test]
fn a_var_is_the_name_the_author_chose_for_code() {
    assert_eq!(
        named(&[("Speed", "speed_kmh")]).names,
        vec![(1, "SPEED_KMH".to_string())]
    );
}

#[test]
fn an_identifier_never_starts_with_a_digit() {
    assert_eq!(named(&[("1st", "")]).names, vec![(1, "_1ST".to_string())]);
}

#[test]
fn a_spelling_two_channels_share_takes_the_second_ones_number() {
    assert_eq!(
        named(&[("x", ""), ("x", "")]).names,
        vec![(1, "X".to_string()), (2, "X_2".to_string())]
    );
}

#[test]
fn a_channel_with_nothing_to_name_it_by_gets_no_constant() {
    assert!(named(&[("", "")]).names.is_empty());
}

#[test]
fn both_writers_declare_the_constant() {
    let model = named(&[("Frame Count", "")]);

    assert!(rust_source(&model, "ch.csv").contains("pub const CH_FRAME_COUNT: u32 = 1;"));
    assert!(c_header(&model, "ch.csv").contains("#define CHDEF_CH_FRAME_COUNT 1"));
}
