//! The C entry points over the same layout `spec_core.rs` uses, so the
//! view rebuilt from pointers and counts answers what the Rust API does
//! (`docs/spec/embedded.md` §4).

#![cfg(feature = "c")]

use chdef_core::c::{
    chdef_core_fill_defaults, chdef_core_read, chdef_core_seal, chdef_core_verify,
    chdef_core_write, ChdefCoreDerived, ChdefCoreLayout,
};
use chdef_core::{Crc, Endian, Range, Slot};

const X25: Crc = Crc {
    width: 16,
    poly: 0x1021,
    init: 0xFFFF,
    refin: true,
    refout: true,
    xorout: 0xFFFF,
};

const SLOTS: [Slot; 2] = [
    Slot {
        number: 1,
        at: 0,
        bytes: 2,
        default: 0x0007,
    },
    Slot {
        number: 2,
        at: 2,
        bytes: 2,
        default: 0,
    },
];

const COVERS: [Range; 1] = [Range { at: 0, len: 2 }];

fn table() -> (ChdefCoreLayout, [ChdefCoreDerived; 1]) {
    let derived = [ChdefCoreDerived {
        slot: 1,
        crc: X25,
        covers: COVERS.as_ptr(),
        cover_count: COVERS.len(),
    }];
    let layout = ChdefCoreLayout {
        slots: SLOTS.as_ptr(),
        slot_count: SLOTS.len(),
        derived: core::ptr::null(),
        derived_count: 1,
        endian: Endian::Little,
        total: 4,
    };
    (layout, derived)
}

/// The layout with its `derived` pointer aimed at `derived`, which cannot
/// be done while building the pair.
fn wire(layout: &mut ChdefCoreLayout, derived: &[ChdefCoreDerived; 1]) {
    layout.derived = derived.as_ptr();
}

#[test]
fn a_frame_filled_sealed_and_verified_through_c_holds_what_rust_would_write() {
    let (mut layout, derived) = table();
    wire(&mut layout, &derived);
    let mut frame = [0u8; 4];

    unsafe {
        assert_eq!(
            chdef_core_fill_defaults(&layout, frame.as_mut_ptr(), frame.len()),
            1
        );
        assert_eq!(chdef_core_verify(&layout, frame.as_ptr(), frame.len()), 0);
        assert_eq!(chdef_core_seal(&layout, frame.as_mut_ptr(), frame.len()), 1);
        assert_eq!(chdef_core_verify(&layout, frame.as_ptr(), frame.len()), 1);

        let mut raw = 0u64;
        assert_eq!(
            chdef_core_read(&layout, frame.as_ptr(), frame.len(), 2, &mut raw),
            1
        );
        assert_eq!(raw, X25.of(&[0x07, 0x00]));
    }
}

#[test]
fn a_short_frame_seals_nothing_and_reports_it() {
    let (mut layout, derived) = table();
    wire(&mut layout, &derived);
    let mut frame = [0x07, 0x00, 0x00];

    unsafe {
        assert_eq!(chdef_core_seal(&layout, frame.as_mut_ptr(), frame.len()), 0);
    }
    assert_eq!(frame, [0x07, 0x00, 0x00]);
}

#[test]
fn a_channel_the_layout_does_not_declare_is_neither_read_nor_written() {
    let (mut layout, derived) = table();
    wire(&mut layout, &derived);
    let mut frame = [0u8; 4];
    let mut raw = 0u64;

    unsafe {
        assert_eq!(
            chdef_core_read(&layout, frame.as_ptr(), frame.len(), 9, &mut raw),
            0
        );
        assert_eq!(
            chdef_core_write(&layout, frame.as_mut_ptr(), frame.len(), 9, 1),
            0
        );
    }
    assert_eq!(frame, [0u8; 4]);
}

#[test]
fn a_null_pointer_is_reported_rather_than_read() {
    let (mut layout, derived) = table();
    wire(&mut layout, &derived);
    let mut raw = 0u64;

    unsafe {
        assert_eq!(
            chdef_core_read(core::ptr::null(), [0u8; 4].as_ptr(), 4, 1, &mut raw),
            0
        );
        assert_eq!(
            chdef_core_read(&layout, core::ptr::null(), 4, 1, &mut raw),
            0
        );
        assert_eq!(
            chdef_core_read(&layout, [0u8; 4].as_ptr(), 4, 1, core::ptr::null_mut()),
            0
        );
        assert_eq!(chdef_core_seal(&layout, core::ptr::null_mut(), 4), 0);
    }
}

#[test]
fn endian_crosses_as_the_int32_the_header_declares() {
    assert_eq!(core::mem::size_of::<Endian>(), 4);
    assert_eq!(Endian::Little as i32, 0);
    assert_eq!(Endian::Big as i32, 1);
}
