//! The core against the specification it implements: `docs/spec/layout.md`
//! for positions and widths, `conversion.md` §3–§4 for raw bytes and
//! defaults, `format.md` §6 for the CRC, and `embedded.md` for what a
//! device is given.
//!
//! The layout used throughout is the one `crates/chdef/vectors/basic`
//! declares, and the frames are that directory's golden vectors, so the
//! core is held to the same bytes every host binding is.

use chdef_core::{read_raw, write_raw, Crc, Derived, Endian, Layout, Range, Slot};

/// The layout of `crates/chdef/vectors/basic/ch.csv`: five channels over
/// thirteen bytes, least significant byte first.
const BASIC: [Slot; 5] = [
    Slot {
        number: 1,
        at: 0,
        bytes: 4,
        default: 0,
    },
    Slot {
        number: 2,
        at: 4,
        bytes: 2,
        default: 0x0001,
    },
    Slot {
        number: 3,
        at: 6,
        bytes: 1,
        default: 0,
    },
    Slot {
        number: 4,
        at: 7,
        bytes: 2,
        default: 0,
    },
    Slot {
        number: 5,
        at: 9,
        bytes: 4,
        default: 0,
    },
];

fn basic() -> Layout<'static> {
    Layout {
        slots: &BASIC,
        derived: &[],
        endian: Endian::Little,
        total: 13,
    }
}

fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `crc16/x25` as `crates/chdef/src/derived.rs` catalogues it.
const X25: Crc = Crc {
    width: 16,
    poly: 0x1021,
    init: 0xFFFF,
    refin: true,
    refout: true,
    xorout: 0xFFFF,
};

#[test]
fn a_frame_reads_as_the_golden_vector_decodes_it() {
    // vectors.txt: D 0100000005000285ffdc050000
    //              1=1/1.0;2=5/5.0;3=2/2.0;4=65413/-12.3;5=1500/1.5
    let layout = basic();
    let frame = bytes("0100000005000285ffdc050000");

    let read: Vec<Option<u64>> = (1..=5).map(|n| layout.read(&frame, n)).collect();
    assert_eq!(read, [Some(1), Some(5), Some(2), Some(65413), Some(1500)]);
}

#[test]
fn a_channel_the_layout_does_not_declare_reads_as_nothing() {
    let frame = bytes("0100000005000285ffdc050000");

    assert_eq!(basic().read(&frame, 9), None);
}

#[test]
fn a_short_frame_drops_the_channels_it_does_not_reach() {
    // vectors.txt: D 0100000005000285ff 1=1/1.0;2=5/5.0;3=2/2.0;4=65413/-12.3
    let layout = basic();
    let frame = bytes("0100000005000285ff");

    assert_eq!(frame.len(), 9);
    assert_eq!(layout.read(&frame, 5), None);
    assert_eq!(layout.read(&frame, 4), Some(65413));
}

#[test]
fn the_defaults_fill_a_frame_as_the_all_defaults_vector_states() {
    // vectors.txt: E - 00000000010000000000000000
    let mut frame = [0u8; 13];

    assert!(basic().fill_defaults(&mut frame));
    assert_eq!(hex(&frame), "00000000010000000000000000");
}

#[test]
fn a_frame_shorter_than_the_layout_takes_no_defaults_at_all() {
    let mut frame = [0u8; 12];

    assert!(!basic().fill_defaults(&mut frame));
    assert_eq!(frame, [0u8; 12]);
}

#[test]
fn a_written_channel_reads_back_least_significant_byte_first() {
    // vectors.txt: E 4=0xFF85 0000000001000085ff00000000
    let layout = basic();
    let mut frame = [0u8; 13];

    assert!(layout.write(&mut frame, 4, 0xFF85));
    assert_eq!(layout.read(&frame, 4), Some(0xFF85));
    assert_eq!(&frame[7..9], &[0x85, 0xFF]);
}

#[test]
fn the_same_channel_written_big_endian_puts_the_high_byte_first() {
    let layout = Layout {
        endian: Endian::Big,
        ..basic()
    };
    let mut frame = [0u8; 13];

    assert!(layout.write(&mut frame, 4, 0xFF85));
    assert_eq!(layout.read(&frame, 4), Some(0xFF85));
    assert_eq!(&frame[7..9], &[0xFF, 0x85]);
}

#[test]
fn bits_above_the_width_of_a_channel_are_cut_rather_than_clamped() {
    // conversion.md §3: writing a raw value is a bit pattern, not a
    // number to be rounded or held to a range.
    let mut out = [0u8; 1];

    assert!(write_raw(&mut out, 1, Endian::Little, 0x1FF));
    assert_eq!(out, [0xFF]);
}

#[test]
fn a_buffer_shorter_than_the_width_takes_nothing() {
    let mut out = [0xAAu8; 1];

    assert!(!write_raw(&mut out, 2, Endian::Little, 0x1234));
    assert_eq!(out, [0xAA]);
}

#[test]
fn eight_bytes_read_as_a_raw_value_in_either_order() {
    let wire = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    assert_eq!(read_raw(&wire, 8, Endian::Little), 0x0807_0605_0403_0201);
    assert_eq!(read_raw(&wire, 8, Endian::Big), 0x0102_0304_0506_0708);
}

#[test]
fn a_declared_width_is_held_to_the_one_to_eight_bytes_the_format_has() {
    let width = |bytes| {
        Slot {
            number: 1,
            at: 0,
            bytes,
            default: 0,
        }
        .width()
    };

    assert_eq!((width(0), width(3), width(9)), (1, 3, 8));
}

#[test]
fn every_catalogued_variant_matches_its_published_check_value() {
    // The self-test each CRC catalogue prints, over the ASCII bytes
    // `123456789`. crates/chdef/src/derived.rs holds the same six.
    let published = [
        (X25, 0x906E),
        (
            Crc {
                width: 16,
                poly: 0x1021,
                init: 0xFFFF,
                refin: false,
                refout: false,
                xorout: 0x0000,
            },
            0x29B1,
        ),
        (
            Crc {
                width: 16,
                poly: 0x1021,
                init: 0x0000,
                refin: true,
                refout: true,
                xorout: 0x0000,
            },
            0x2189,
        ),
        (
            Crc {
                width: 16,
                poly: 0x1021,
                init: 0x0000,
                refin: false,
                refout: false,
                xorout: 0x0000,
            },
            0x31C3,
        ),
        (
            Crc {
                width: 8,
                poly: 0x07,
                init: 0x00,
                refin: false,
                refout: false,
                xorout: 0x00,
            },
            0xF4,
        ),
        (
            Crc {
                width: 32,
                poly: 0x04C1_1DB7,
                init: 0xFFFF_FFFF,
                refin: true,
                refout: true,
                xorout: 0xFFFF_FFFF,
            },
            0xCBF4_3926,
        ),
    ];

    for (crc, check) in published {
        assert_eq!(crc.check(), check, "{crc:?}");
    }
}

/// Two channels over four bytes, the second holding a CRC of the first.
const SEALED_SLOTS: [Slot; 2] = [
    Slot {
        number: 1,
        at: 0,
        bytes: 2,
        default: 0,
    },
    Slot {
        number: 2,
        at: 2,
        bytes: 2,
        default: 0,
    },
];

const SEALED_COVERS: [Range; 1] = [Range { at: 0, len: 2 }];

fn sealed() -> Layout<'static> {
    const DERIVED: [Derived<'static>; 1] = [Derived {
        slot: 1,
        crc: X25,
        covers: &SEALED_COVERS,
    }];
    Layout {
        slots: &SEALED_SLOTS,
        derived: &DERIVED,
        endian: Endian::Little,
        total: 4,
    }
}

#[test]
fn a_frame_that_was_never_sealed_does_not_verify() {
    let layout = sealed();
    let frame = [0x07, 0x00, 0x00, 0x00];

    assert!(!layout.verify(&frame));
}

#[test]
fn sealing_writes_the_crc_of_the_covered_bytes_and_then_it_verifies() {
    let layout = sealed();
    let mut frame = [0x07, 0x00, 0x00, 0x00];

    assert!(layout.seal(&mut frame));
    assert_eq!(layout.read(&frame, 2), Some(X25.of(&[0x07, 0x00])));
    assert!(layout.verify(&frame));
}

#[test]
fn a_frame_too_short_for_the_derived_slot_is_not_sealed_at_all() {
    let layout = sealed();
    let mut frame = [0x07, 0x00, 0x00];

    assert!(!layout.seal(&mut frame));
    assert_eq!(frame, [0x07, 0x00, 0x00]);
    assert!(!layout.verify(&frame));
}

#[test]
fn a_recipe_covering_bytes_the_frame_does_not_have_computes_nothing() {
    const COVERS: [Range; 1] = [Range { at: 0, len: 8 }];
    let derived = Derived {
        slot: 1,
        crc: X25,
        covers: &COVERS,
    };
    let layout = Layout {
        slots: &SEALED_SLOTS,
        derived: &[],
        endian: Endian::Little,
        total: 4,
    };
    let frame = [0x07, 0x00, 0x00, 0x00];

    assert_eq!(layout.derived_value(&frame, &derived), None);
}

#[test]
fn a_recipe_over_several_stretches_is_the_crc_of_them_joined() {
    // The host concatenates the covered bytes and calls `Crc::of`; a
    // device has no buffer to concatenate into and feeds the stretches one
    // after another. Both must reach the same value.
    const SLOTS: [Slot; 1] = [Slot {
        number: 1,
        at: 6,
        bytes: 2,
        default: 0,
    }];
    const COVERS: [Range; 2] = [Range { at: 0, len: 2 }, Range { at: 3, len: 3 }];
    let derived = Derived {
        slot: 0,
        crc: X25,
        covers: &COVERS,
    };
    let layout = Layout {
        slots: &SLOTS,
        derived: &[],
        endian: Endian::Little,
        total: 8,
    };
    let frame = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x00, 0x00];

    assert_eq!(
        layout.derived_value(&frame, &derived),
        Some(X25.of(&[0x11, 0x22, 0x44, 0x55, 0x66]))
    );
}
