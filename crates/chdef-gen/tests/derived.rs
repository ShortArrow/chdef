//! A derived channel survives the trip into the table: the recipe becomes
//! six numbers and byte ranges, and the core computes from them what the
//! host computes from the definition (`docs/spec/format.md` §6).

use chdef::ColumnVocabulary;
use chdef_core::{Crc, Endian, Range};
use chdef_gen::{model, Model, Refusal};

const X25: Crc = Crc {
    width: 16,
    poly: 0x1021,
    init: 0xFFFF,
    refin: true,
    refout: true,
    xorout: 0xFFFF,
};

fn definition(recipe: &str) -> String {
    format!(
        "number,bytes,type,kind,derived,name\n\
         1,2,UI,plain,,speed\n\
         2,2,UI,derived,{recipe},crc\n"
    )
}

fn built(recipe: &str) -> Result<Model, Refusal> {
    model(
        definition(recipe).as_bytes(),
        b"",
        Endian::Little,
        &ColumnVocabulary::new(),
    )
}

#[test]
fn a_recipe_becomes_the_slot_it_fills_and_the_bytes_it_covers() {
    let model = built("crc16/x25 1..1").expect("the definition loads");

    assert_eq!(model.derived.len(), 1);
    assert_eq!(model.derived[0].slot, 1);
    assert_eq!(model.derived[0].crc, X25);
    assert_eq!(model.derived[0].covers, vec![Range { at: 0, len: 2 }]);
}

#[test]
fn the_core_seals_the_frame_the_table_describes() {
    let model = built("crc16/x25 1..1").expect("the definition loads");
    let view = model.layout();
    let layout = view.as_layout();
    let mut frame = [0x07u8, 0x00, 0x00, 0x00];

    assert!(!layout.verify(&frame), "an unsealed frame verifies");
    assert!(layout.seal(&mut frame), "the frame could not be sealed");

    assert_eq!(layout.read(&frame, 2), Some(X25.of(&[0x07, 0x00])));
    assert!(layout.verify(&frame), "a sealed frame does not verify");
}

#[test]
fn a_recipe_covering_a_channel_the_layout_lacks_is_refused() {
    match built("crc16/x25 1..3") {
        Err(Refusal::Coverage { channel, covers }) => assert_eq!((channel, covers), (2, 3)),
        other => panic!("expected a coverage refusal, got {other:?}"),
    }
}

#[test]
fn a_recipe_this_chdef_does_not_compute_is_refused() {
    match built("fletcher16 1..1") {
        Err(Refusal::Recipe { channel, name }) => {
            assert_eq!(channel, 2);
            assert_eq!(name, "fletcher16");
        }
        other => panic!("expected a recipe refusal, got {other:?}"),
    }
}

#[test]
fn the_numbers_of_a_recipe_reach_the_table_as_written() {
    // A device whose CRC is in no catalogue writes the numbers instead
    // (ADR-0029), and the table must carry those, not a nearby name.
    let model = built("crc16 poly=0x8005 init=0x0000 refin=1 refout=1 xorout=0x0000 1..1")
        .expect("the definition loads");

    assert_eq!(
        model.derived[0].crc,
        Crc {
            width: 16,
            poly: 0x8005,
            init: 0,
            refin: true,
            refout: true,
            xorout: 0,
        }
    );
}
