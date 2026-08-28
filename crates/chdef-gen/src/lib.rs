//! A definition file expanded into the constant table a device holds
//! (`docs/spec/embedded.md` §3).
//!
//! The host reads CH / BF CSV; a microcontroller does not. What crosses
//! the gap is a table: every channel's number, position, width and merged
//! default, and every derived channel's slot, CRC and covered byte ranges,
//! already resolved from channel numbers to offsets. Names, units, `lsb`,
//! `offset` and the rest stay behind — the target never uses them.
//!
//! A definition with any Issue is refused. A row the host would load with
//! a warning does not reach a device, where nothing can warn.

mod c_table;
mod rust_table;

use std::fmt;

use chdef::{ChdefError, ColumnVocabulary, Derivation, Issue};
use chdef_core::{Crc, Derived, Endian, Layout, Range, Slot};

pub use c_table::c_header;
pub use rust_table::rust_source;

/// The table a device holds, in the terms `chdef-core` states it.
///
/// `names` pairs a channel number with the identifier the per-channel
/// constant takes; a channel with neither `var` nor `name` has no entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub slots: Vec<Slot>,
    pub derived: Vec<DerivedModel>,
    pub endian: Endian,
    pub total: u32,
    pub names: Vec<(u32, String)>,
}

/// One derived channel of the table: the slot it fills as an index into
/// [`Model::slots`], the six CRC numbers, and the stretches it covers in
/// the order the recipe wrote them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedModel {
    pub slot: u32,
    pub crc: Crc,
    pub covers: Vec<Range>,
}

/// Why a definition did not become a table.
#[derive(Debug)]
pub enum Refusal {
    /// The definition loads with findings. A device has nowhere to report
    /// one, so every finding refuses it.
    Issues(Vec<Issue>),
    /// A recipe naming an algorithm this chdef does not compute. A device
    /// cannot be handed a checksum nothing can run.
    Recipe { channel: u32, name: String },
    /// A recipe covering a channel the layout does not have, so the
    /// stretch it covers has no offset.
    Coverage { channel: u32, covers: u32 },
    /// The bytes are not a CSV that can be split into records at all.
    Unreadable(ChdefError),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::Issues(issues) => {
                for (index, issue) in issues.iter().enumerate() {
                    if index > 0 {
                        writeln!(f)?;
                    }
                    write!(f, "{}{}: {}", issue.code, where_of(issue), issue.message)?;
                }
                Ok(())
            }
            Refusal::Recipe { channel, name } => write!(
                f,
                "the recipe of channel {channel} is `{name}`, which chdef does not compute; \
                 a device has no way to run it."
            ),
            Refusal::Coverage { channel, covers } => write!(
                f,
                "the recipe of channel {channel} covers channel {covers}, \
                 which the layout does not have."
            ),
            Refusal::Unreadable(error) => write!(f, "{error}"),
        }
    }
}

/// ` (row 3, col 1)` — whichever of the two the finding carries.
fn where_of(issue: &Issue) -> String {
    match (issue.row, issue.col) {
        (Some(row), Some(col)) => format!(" (row {row}, col {col})"),
        (Some(row), None) => format!(" (row {row})"),
        (None, Some(col)) => format!(" (col {col})"),
        (None, None) => String::new(),
    }
}

/// The table a CH CSV and an optional BF CSV describe, or why they
/// describe none.
///
/// Bytes rather than text: the CLI hands over what it read, and the
/// `_bytes_with` entry points strip the BOMs a spreadsheet writes. Pass an
/// empty slice for a definition with no BF file.
pub fn model(
    ch_csv: &[u8],
    bf_csv: &[u8],
    endian: Endian,
    vocabulary: &ColumnVocabulary,
) -> Result<Model, Refusal> {
    let channels =
        chdef::parse_ch_csv_bytes_with(ch_csv, vocabulary).map_err(Refusal::Unreadable)?;
    let bitfields =
        chdef::parse_bf_csv_bytes_with(bf_csv, vocabulary).map_err(Refusal::Unreadable)?;

    let mut issues = channels.issues;
    issues.extend(bitfields.issues);
    let built = chdef::build_layout(channels.value, bitfields.value);
    issues.extend(built.issues);
    if !issues.is_empty() {
        return Err(Refusal::Issues(issues));
    }
    let layout = built.value;

    let slots: Vec<Slot> = layout
        .positions()
        .map(|(at, ch)| Slot {
            number: ch.number,
            at: at as u32,
            bytes: ch.width() as u8,
            default: layout.channel_default(ch.number).unwrap_or(0),
        })
        .collect();

    let mut derived = Vec::new();
    for (index, ch) in layout.channels.iter().enumerate() {
        let Some(recipe) = ch.derived.as_ref() else {
            continue;
        };
        let crc = match &recipe.derivation {
            Derivation::Crc(crc) => *crc,
            // A recipe chdef cannot compute is one a device cannot
            // either, and the enum can grow: an arm that is not there yet
            // is refused by the name it prints.
            Derivation::Unknown(name) => {
                return Err(Refusal::Recipe {
                    channel: ch.number,
                    name: name.clone(),
                })
            }
            other => {
                return Err(Refusal::Recipe {
                    channel: ch.number,
                    name: format!("{other:?}"),
                })
            }
        };
        let mut covers = Vec::new();
        for (low, high) in &recipe.spans {
            for number in *low..=*high {
                let slot =
                    slots
                        .iter()
                        .find(|slot| slot.number == number)
                        .ok_or(Refusal::Coverage {
                            channel: ch.number,
                            covers: number,
                        })?;
                covers.push(Range {
                    at: slot.at,
                    len: slot.width() as u32,
                });
            }
        }
        derived.push(DerivedModel {
            slot: index as u32,
            crc,
            covers,
        });
    }

    Ok(Model {
        names: identifiers(&layout),
        slots,
        derived,
        endian,
        total: layout.total_bytes() as u32,
    })
}

/// The identifier each channel's constant takes: its `var`, or its `name`
/// where there is no `var`, upper-cased and cut down to what an identifier
/// may hold. A channel with neither gets no constant; a spelling two
/// channels share takes the second one's number.
fn identifiers(layout: &chdef::ChannelLayout) -> Vec<(u32, String)> {
    let mut names: Vec<(u32, String)> = Vec::new();
    for ch in &layout.channels {
        let source = if ch.var.trim().is_empty() {
            &ch.name
        } else {
            &ch.var
        };
        let Some(identifier) = identifier(source) else {
            continue;
        };
        let identifier = if names.iter().any(|(_, taken)| *taken == identifier) {
            format!("{identifier}_{}", ch.number)
        } else {
            identifier
        };
        names.push((ch.number, identifier));
    }
    names
}

/// One spelling as an identifier, or `None` when nothing of it survives.
fn identifier(source: &str) -> Option<String> {
    let mut out = String::new();
    for ch in source.to_uppercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        return None;
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        Some(format!("_{out}"))
    } else {
        Some(out.to_string())
    }
}

/// A [`Model`] lent out as the [`Layout`] the core takes: the borrowed
/// tables the constant would hold, without generating a line of it.
pub struct LayoutView<'a> {
    slots: &'a [Slot],
    derived: Vec<Derived<'a>>,
    endian: Endian,
    total: u32,
}

impl LayoutView<'_> {
    /// The layout itself, borrowing this view.
    pub fn as_layout(&self) -> Layout<'_> {
        Layout {
            slots: self.slots,
            derived: &self.derived,
            endian: self.endian,
            total: self.total,
        }
    }
}

impl Model {
    /// This table as the core reads it, so the same rules answer here as
    /// on the device.
    pub fn layout(&self) -> LayoutView<'_> {
        LayoutView {
            slots: &self.slots,
            derived: self
                .derived
                .iter()
                .map(|d| Derived {
                    slot: d.slot,
                    crc: d.crc,
                    covers: &d.covers,
                })
                .collect(),
            endian: self.endian,
            total: self.total,
        }
    }
}
