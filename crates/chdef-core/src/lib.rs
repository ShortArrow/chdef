//! The rules a device needs from chdef, and nothing else
//! (`docs/spec/embedded.md`).
//!
//! A microcontroller does not read a definition file: it holds the layout
//! the file describes, fixed when the firmware was built (ADR-0034). What
//! it needs is where each channel sits, its raw value out of a frame and
//! into one, the default it starts from, and the CRC a derived channel
//! computes over the bytes it covers. Parsing, vocabularies, diagnostics,
//! physical values and the grid stay on the host, in `chdef`.
//!
//! Nothing here allocates, uses floating point or panics on any input.

#![no_std]
#![deny(unsafe_code)]

#[cfg(feature = "c")]
pub mod c;

/// The order a channel's bytes sit in on the wire
/// (`docs/spec/layout.md` §2). The whole frame takes one of the two;
/// a channel does not choose its own.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Endian {
    /// Least significant byte first, at the channel's lowest offset.
    #[default]
    Little = 0,
    /// Most significant byte first, at the channel's lowest offset.
    Big = 1,
}

/// One channel of the layout: where it sits, how wide it is, and the value
/// a frame starts from.
///
/// `bytes` is 1..=8; a wider value counts as 8 and 0 as 1
/// (`docs/spec/layout.md`). `default` is already merged with the channel's
/// BF rows (`docs/spec/conversion.md` §4) when the table was generated.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The channel number the definition gave it.
    pub number: u32,
    /// The offset of its first byte in the frame.
    pub at: u32,
    /// Its declared width in bytes, before clamping.
    pub bytes: u8,
    /// Its raw default, with its BF rows merged in.
    pub default: u64,
}

impl Slot {
    /// The width this channel actually occupies: `bytes` held to 1..=8, the
    /// range the format defines (`docs/spec/layout.md`).
    pub const fn width(&self) -> usize {
        match self.bytes {
            0 => 1,
            1..=8 => self.bytes as usize,
            _ => 8,
        }
    }
}

/// The Rocksoft model: the six numbers that decide a CRC completely.
///
/// `refin` reflects each input byte before it enters the register and
/// `refout` reflects the register at the end, which together are what a
/// right-shifting implementation does.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crc {
    /// Bits in the result: 8, 16, 32 or 64.
    pub width: u32,
    /// The generator polynomial, top bit implicit.
    pub poly: u64,
    /// The register's starting value.
    pub init: u64,
    /// Whether each input byte is reflected.
    pub refin: bool,
    /// Whether the final register is reflected.
    pub refout: bool,
    /// The value XORed into the final register.
    pub xorout: u64,
}

impl Crc {
    /// The check value of this parameter set: its CRC of the ASCII bytes
    /// `123456789`, the self-test every CRC catalogue prints.
    pub fn check(self) -> u64 {
        self.of(b"123456789")
    }

    /// The CRC of `data`.
    pub fn of(self, data: &[u8]) -> u64 {
        let top = 1u64 << (self.width - 1);
        let mask = if self.width >= 64 {
            u64::MAX
        } else {
            (1u64 << self.width) - 1
        };

        let mut reg = self.init & mask;
        for byte in data {
            let byte = if self.refin {
                reflect(*byte as u64, 8)
            } else {
                *byte as u64
            };
            reg ^= byte << (self.width - 8);
            for _ in 0..8 {
                reg = if reg & top != 0 {
                    ((reg << 1) ^ self.poly) & mask
                } else {
                    (reg << 1) & mask
                };
            }
        }
        if self.refout {
            reg = reflect(reg, self.width);
        }
        (reg ^ self.xorout) & mask
    }
}

fn reflect(mut value: u64, width: u32) -> u64 {
    let mut out = 0;
    for _ in 0..width {
        out = (out << 1) | (value & 1);
        value >>= 1;
    }
    out
}

/// The register arithmetic of [`Crc::of`], reachable one stretch of bytes
/// at a time.
///
/// A recipe covers several ranges of a frame, which a host concatenates
/// into one buffer before calling [`Crc::of`]. A device has no buffer to
/// concatenate into, so it starts the register once, feeds each range in
/// recipe order, and finishes: the same bytes reach the register in the
/// same order, so the result is the same. `spec_core.rs` holds the two
/// against each other.
///
/// Every shift here is held below 64, so a nonsense `width` from a
/// hand-written table gives a nonsense value rather than a panic.
impl Crc {
    /// The mask that cuts a register to `width` bits.
    fn mask(self) -> u64 {
        if self.width >= 64 {
            u64::MAX
        } else {
            (1u64 << self.width) - 1
        }
    }

    /// The register before any byte has entered it.
    fn start(self) -> u64 {
        self.init & self.mask()
    }

    /// The register after `data` has passed through it.
    fn fold(self, mut reg: u64, data: &[u8]) -> u64 {
        let mask = self.mask();
        let top = 1u64 << self.width.saturating_sub(1).min(63);
        let entry = self.width.saturating_sub(8).min(63);
        for byte in data {
            let byte = if self.refin {
                reflect(*byte as u64, 8)
            } else {
                *byte as u64
            };
            reg ^= byte << entry;
            for _ in 0..8 {
                reg = if reg & top != 0 {
                    ((reg << 1) ^ self.poly) & mask
                } else {
                    (reg << 1) & mask
                };
            }
        }
        reg
    }

    /// The register turned into the value the recipe states.
    fn finish(self, mut reg: u64) -> u64 {
        if self.refout {
            reg = reflect(reg, self.width.min(64));
        }
        (reg ^ self.xorout) & self.mask()
    }
}

/// A stretch of the frame a recipe covers, resolved from channel numbers to
/// an offset and a length when the table was generated.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    /// The offset of its first byte in the frame.
    pub at: u32,
    /// How many bytes it covers.
    pub len: u32,
}

/// A channel chdef computes from the rest of the frame
/// (`docs/spec/format.md` §6): the slot it fills, the CRC it runs, and the
/// stretches it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Derived<'a> {
    /// Which of [`Layout::slots`] holds the result, as an index into it —
    /// not the channel number.
    pub slot: u32,
    /// The six numbers of the recipe.
    pub crc: Crc,
    /// The stretches it covers, in the order the recipe wrote them.
    pub covers: &'a [Range],
}

/// A frame's layout as the firmware holds it: every channel's position and
/// default, every derived channel's recipe, the byte order and the total.
///
/// `chdef-gen` writes one of these as a constant from a definition file. It
/// borrows its tables rather than owning them, so it costs no allocation
/// and can live in flash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout<'a> {
    /// Every channel, in the order the definition declared them.
    pub slots: &'a [Slot],
    /// Every derived channel, in declaration order.
    pub derived: &'a [Derived<'a>],
    /// The byte order the whole frame takes.
    pub endian: Endian,
    /// The frame's length in bytes.
    pub total: u32,
}

/// The raw bit pattern held in the low `width` bytes of `bytes` (at most
/// `bytes.len()` of them), read in the given byte order
/// (`docs/spec/conversion.md` §3).
///
/// A short slice is read as far as it goes rather than refused; the caller
/// decides whether that is a channel at all.
pub fn read_raw(bytes: &[u8], width: usize, endian: Endian) -> u64 {
    let n = bytes.len().min(width);
    let Some(head) = bytes.get(..n) else {
        return 0;
    };
    let mut raw = 0u64;
    match endian {
        Endian::Little => {
            for &b in head.iter().rev() {
                raw = (raw << 8) | b as u64;
            }
        }
        Endian::Big => {
            for &b in head {
                raw = (raw << 8) | b as u64;
            }
        }
    }
    raw
}

/// The inverse of [`read_raw`]: the low `width` bytes of `raw` into
/// `out[..width]`, in the given byte order.
///
/// Bits above `width` bytes are cut. Nothing is rounded, nothing is
/// clamped and nothing is reported: the value is a bit pattern, and the
/// caller decides what patterns mean. Returns `false`, having written
/// nothing, when `out` is shorter than `width`.
pub fn write_raw(out: &mut [u8], width: usize, endian: Endian, raw: u64) -> bool {
    if out.len() < width {
        return false;
    }
    for (i, cell) in out.iter_mut().enumerate().take(width) {
        let place = match endian {
            Endian::Little => i,
            Endian::Big => width - 1 - i,
        };
        *cell = match place.checked_mul(8) {
            Some(bits) if bits < 64 => (raw >> bits) as u8,
            _ => 0,
        };
    }
    true
}

/// The stretch of `frame` a slot occupies, or `None` when the frame ends
/// before the slot does.
fn slot_bytes<'f>(frame: &'f [u8], slot: &Slot) -> Option<&'f [u8]> {
    let at = slot.at as usize;
    let end = at.checked_add(slot.width())?;
    frame.get(at..end)
}

/// The value a derived channel computes over the bytes it covers, once its
/// own slot is known to fit. Both the Rust and the C entry points reach a
/// derived channel through this one function.
fn derived_value_of(slots: &[Slot], frame: &[u8], derived: &Derived) -> Option<u64> {
    let slot = slots.get(derived.slot as usize)?;
    slot_bytes(frame, slot)?;
    let mut reg = derived.crc.start();
    for range in derived.covers {
        let at = range.at as usize;
        let end = at.checked_add(range.len as usize)?;
        let covered = frame.get(at..end)?;
        reg = derived.crc.fold(reg, covered);
    }
    Some(derived.crc.finish(reg))
}

/// One derived channel computed and written into its slot.
fn seal_one(slots: &[Slot], endian: Endian, frame: &mut [u8], derived: &Derived) -> bool {
    let Some(value) = derived_value_of(slots, frame, derived) else {
        return false;
    };
    let Some(slot) = slots.get(derived.slot as usize) else {
        return false;
    };
    let (at, width) = (slot.at as usize, slot.width());
    let Some(end) = at.checked_add(width) else {
        return false;
    };
    let Some(out) = frame.get_mut(at..end) else {
        return false;
    };
    write_raw(out, width, endian, value)
}

/// Whether one derived channel's slot already holds what it computes.
fn verify_one(slots: &[Slot], endian: Endian, frame: &[u8], derived: &Derived) -> bool {
    let Some(value) = derived_value_of(slots, frame, derived) else {
        return false;
    };
    let Some(slot) = slots.get(derived.slot as usize) else {
        return false;
    };
    let Some(held) = slot_bytes(frame, slot) else {
        return false;
    };
    read_raw(held, slot.width(), endian) == value
}

impl<'a> Layout<'a> {
    /// The channel with this number, or `None` when the layout has none.
    pub fn slot(&self, number: u32) -> Option<&'a Slot> {
        self.slots.iter().find(|slot| slot.number == number)
    }

    /// One channel's raw value out of a frame. `None` when the channel is
    /// unknown or `frame` ends before the channel does — a short frame
    /// drops the channels it does not reach rather than zero-filling them.
    pub fn read(&self, frame: &[u8], number: u32) -> Option<u64> {
        let slot = self.slot(number)?;
        let bytes = slot_bytes(frame, slot)?;
        Some(read_raw(bytes, slot.width(), self.endian))
    }

    /// One channel's raw value into a frame. `false`, with nothing
    /// written, when the channel is unknown or the frame ends before it
    /// does.
    pub fn write(&self, frame: &mut [u8], number: u32, raw: u64) -> bool {
        let Some(slot) = self.slot(number) else {
            return false;
        };
        let (at, width) = (slot.at as usize, slot.width());
        let Some(end) = at.checked_add(width) else {
            return false;
        };
        let Some(out) = frame.get_mut(at..end) else {
            return false;
        };
        write_raw(out, width, self.endian, raw)
    }

    /// Every channel's default into the frame, the state a frame starts
    /// from (`docs/spec/conversion.md` §4). Bytes no channel occupies are
    /// left as they were. `false`, with nothing written, when the frame is
    /// shorter than `total` or than any channel in it.
    pub fn fill_defaults(&self, frame: &mut [u8]) -> bool {
        if frame.len() < self.total as usize {
            return false;
        }
        for slot in self.slots {
            if slot_bytes(frame, slot).is_none() {
                return false;
            }
        }
        for slot in self.slots {
            let (at, width) = (slot.at as usize, slot.width());
            let Some(end) = at.checked_add(width) else {
                return false;
            };
            let Some(out) = frame.get_mut(at..end) else {
                return false;
            };
            write_raw(out, width, self.endian, slot.default);
        }
        true
    }

    /// The value a derived channel computes over the bytes it covers.
    /// `None` when any covered stretch, or the slot the result goes into,
    /// ends past the frame.
    pub fn derived_value(&self, frame: &[u8], derived: &Derived) -> Option<u64> {
        derived_value_of(self.slots, frame, derived)
    }

    /// Every derived channel computed and written, in declaration order.
    /// `false` when any of them cannot be, and then nothing is written at
    /// all: a frame is sealed completely or not at all.
    pub fn seal(&self, frame: &mut [u8]) -> bool {
        for derived in self.derived {
            if derived_value_of(self.slots, frame, derived).is_none() {
                return false;
            }
        }
        for derived in self.derived {
            if !seal_one(self.slots, self.endian, frame, derived) {
                return false;
            }
        }
        true
    }

    /// Whether every derived channel holds the value it computes. `false`
    /// when any of them cannot be computed at all.
    pub fn verify(&self, frame: &[u8]) -> bool {
        self.derived
            .iter()
            .all(|derived| verify_one(self.slots, self.endian, frame, derived))
    }
}
