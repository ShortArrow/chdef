//! The core's operations as C entry points
//! (`docs/spec/embedded.md` §4).
//!
//! A C firmware links this crate as a static library and passes the
//! `CHDEF_LAYOUT` that `chdef-gen` wrote. The table's shape is the Rust
//! one with each slice split into a pointer and a count, because C has no
//! slice: [`ChdefCoreLayout`] and [`ChdefCoreDerived`] carry the halves,
//! and every call rebuilds the borrowed view before handing the work to
//! the same functions the Rust API calls.
//!
//! Every call returns `1` on success and `0` when a pointer is null, the
//! frame is shorter than the layout, or the channel is not in it. No other
//! status exists: the definition was checked when the table was generated,
//! so there is no diagnostic left to carry.

#![allow(unsafe_code)]

use crate::{derived_value_of, seal_one, verify_one, Crc, Derived, Endian, Layout, Range, Slot};

/// A derived channel as C holds it: [`Derived`] with its `covers` slice
/// split into a pointer and a count.
#[repr(C)]
pub struct ChdefCoreDerived {
    /// Which of the layout's slots holds the result, as an index into the
    /// slot table.
    pub slot: u32,
    /// The six numbers of the recipe.
    pub crc: Crc,
    /// The stretches it covers, in recipe order.
    pub covers: *const Range,
    /// How many stretches `covers` points at.
    pub cover_count: usize,
}

/// A frame's layout as C holds it: [`Layout`] with each slice split into a
/// pointer and a count.
#[repr(C)]
pub struct ChdefCoreLayout {
    /// Every channel, in declaration order.
    pub slots: *const Slot,
    /// How many channels `slots` points at.
    pub slot_count: usize,
    /// Every derived channel, in declaration order.
    pub derived: *const ChdefCoreDerived,
    /// How many derived channels `derived` points at.
    pub derived_count: usize,
    /// The byte order the whole frame takes.
    pub endian: Endian,
    /// The frame's length in bytes.
    pub total: u32,
}

/// The slot table as a slice, or `None` when the pointer is null and the
/// count is not zero.
///
/// # Safety
///
/// `layout.slots` must point at `layout.slot_count` initialised [`Slot`]s
/// that outlive `'a`.
unsafe fn slots_of<'a>(layout: &ChdefCoreLayout) -> Option<&'a [Slot]> {
    if layout.slot_count == 0 {
        return Some(&[]);
    }
    if layout.slots.is_null() {
        return None;
    }
    Some(core::slice::from_raw_parts(layout.slots, layout.slot_count))
}

/// One derived channel of the C table as the Rust [`Derived`] the core
/// works with.
///
/// # Safety
///
/// `index` must be below `layout.derived_count`, and the entry's `covers`
/// must point at `cover_count` initialised [`Range`]s that outlive `'a`.
unsafe fn derived_of<'a>(layout: &ChdefCoreLayout, index: usize) -> Option<Derived<'a>> {
    if layout.derived.is_null() {
        return None;
    }
    let entry = &*layout.derived.add(index);
    let covers: &[Range] = if entry.cover_count == 0 {
        &[]
    } else if entry.covers.is_null() {
        return None;
    } else {
        core::slice::from_raw_parts(entry.covers, entry.cover_count)
    };
    Some(Derived {
        slot: entry.slot,
        crc: entry.crc,
        covers,
    })
}

/// The frame as a shared slice, or `None` when the pointer is null and the
/// length is not zero.
///
/// # Safety
///
/// `frame` must point at `len` initialised bytes that outlive `'a`.
unsafe fn frame_of<'a>(frame: *const u8, len: usize) -> Option<&'a [u8]> {
    if len == 0 {
        return Some(&[]);
    }
    if frame.is_null() {
        return None;
    }
    Some(core::slice::from_raw_parts(frame, len))
}

/// The frame as an exclusive slice, or `None` when the pointer is null and
/// the length is not zero.
///
/// # Safety
///
/// `frame` must point at `len` initialised bytes that outlive `'a` and be
/// reachable through no other pointer for that time.
unsafe fn frame_mut_of<'a>(frame: *mut u8, len: usize) -> Option<&'a mut [u8]> {
    if len == 0 {
        return Some(&mut []);
    }
    if frame.is_null() {
        return None;
    }
    Some(core::slice::from_raw_parts_mut(frame, len))
}

/// The layout without its derived channels, which `read`, `write` and
/// `fill_defaults` do not consult.
fn view<'a>(layout: &ChdefCoreLayout, slots: &'a [Slot]) -> Layout<'a> {
    Layout {
        slots,
        derived: &[],
        endian: layout.endian,
        total: layout.total,
    }
}

/// One channel's raw value out of a frame, into `*out_raw`.
///
/// Returns `1` on success and `0` when any pointer is null, the channel is
/// not in the layout, or the frame ends before the channel does.
///
/// # Safety
///
/// `layout` must point at a table whose `slots` and `derived` arrays hold
/// the counts beside them; `frame` must point at `len` readable bytes; and
/// `out_raw` must point at a writable `uint64_t`.
#[no_mangle]
pub unsafe extern "C" fn chdef_core_read(
    layout: *const ChdefCoreLayout,
    frame: *const u8,
    len: usize,
    number: u32,
    out_raw: *mut u64,
) -> i32 {
    let (Some(layout), false) = (layout.as_ref(), out_raw.is_null()) else {
        return 0;
    };
    let (Some(slots), Some(frame)) = (slots_of(layout), frame_of(frame, len)) else {
        return 0;
    };
    match view(layout, slots).read(frame, number) {
        Some(raw) => {
            *out_raw = raw;
            1
        }
        None => 0,
    }
}

/// One channel's raw value into a frame.
///
/// Returns `1` on success and `0` when any pointer is null, the channel is
/// not in the layout, or the frame ends before the channel does; then
/// nothing is written.
///
/// # Safety
///
/// `layout` must point at a table whose `slots` and `derived` arrays hold
/// the counts beside them, and `frame` must point at `len` writable bytes
/// reachable through no other pointer for the call.
#[no_mangle]
pub unsafe extern "C" fn chdef_core_write(
    layout: *const ChdefCoreLayout,
    frame: *mut u8,
    len: usize,
    number: u32,
    raw: u64,
) -> i32 {
    let Some(layout) = layout.as_ref() else {
        return 0;
    };
    let (Some(slots), Some(frame)) = (slots_of(layout), frame_mut_of(frame, len)) else {
        return 0;
    };
    i32::from(view(layout, slots).write(frame, number, raw))
}

/// Every channel's default into a frame.
///
/// Returns `1` on success and `0` when any pointer is null or the frame is
/// shorter than the layout; then nothing is written.
///
/// # Safety
///
/// `layout` must point at a table whose `slots` and `derived` arrays hold
/// the counts beside them, and `frame` must point at `len` writable bytes
/// reachable through no other pointer for the call.
#[no_mangle]
pub unsafe extern "C" fn chdef_core_fill_defaults(
    layout: *const ChdefCoreLayout,
    frame: *mut u8,
    len: usize,
) -> i32 {
    let Some(layout) = layout.as_ref() else {
        return 0;
    };
    let (Some(slots), Some(frame)) = (slots_of(layout), frame_mut_of(frame, len)) else {
        return 0;
    };
    i32::from(view(layout, slots).fill_defaults(frame))
}

/// Every derived channel computed and written, in declaration order.
///
/// Returns `1` on success and `0` when any pointer is null or any derived
/// channel cannot be computed; then nothing is written at all.
///
/// # Safety
///
/// `layout` must point at a table whose `slots` and `derived` arrays hold
/// the counts beside them and whose every derived entry's `covers` holds
/// its own, and `frame` must point at `len` writable bytes reachable
/// through no other pointer for the call.
#[no_mangle]
pub unsafe extern "C" fn chdef_core_seal(
    layout: *const ChdefCoreLayout,
    frame: *mut u8,
    len: usize,
) -> i32 {
    let Some(layout) = layout.as_ref() else {
        return 0;
    };
    let (Some(slots), Some(frame)) = (slots_of(layout), frame_mut_of(frame, len)) else {
        return 0;
    };
    for index in 0..layout.derived_count {
        let Some(derived) = derived_of(layout, index) else {
            return 0;
        };
        if derived_value_of(slots, frame, &derived).is_none() {
            return 0;
        }
    }
    for index in 0..layout.derived_count {
        let Some(derived) = derived_of(layout, index) else {
            return 0;
        };
        if !seal_one(slots, layout.endian, frame, &derived) {
            return 0;
        }
    }
    1
}

/// Whether every derived channel holds the value it computes.
///
/// Returns `1` when they all do, and `0` when any pointer is null, any
/// derived channel cannot be computed, or any holds something else.
///
/// # Safety
///
/// `layout` must point at a table whose `slots` and `derived` arrays hold
/// the counts beside them and whose every derived entry's `covers` holds
/// its own, and `frame` must point at `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn chdef_core_verify(
    layout: *const ChdefCoreLayout,
    frame: *const u8,
    len: usize,
) -> i32 {
    let Some(layout) = layout.as_ref() else {
        return 0;
    };
    let (Some(slots), Some(frame)) = (slots_of(layout), frame_of(frame, len)) else {
        return 0;
    };
    for index in 0..layout.derived_count {
        let Some(derived) = derived_of(layout, index) else {
            return 0;
        };
        if !verify_one(slots, layout.endian, frame, &derived) {
            return 0;
        }
    }
    1
}

/// Where a panic ends on a bare target: nowhere to report it and nothing
/// to unwind, so the device stops.
///
/// Only bare targets get this. A host test links the standard library's
/// handler, and Rust firmware that already declares its own keeps it.
#[cfg(all(feature = "c", not(test), target_os = "none"))]
#[panic_handler]
fn halt(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
