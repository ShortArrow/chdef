//! Channels chdef computes from the rest of the frame
//! (`docs/spec/format.md` §6).
//!
//! A recipe is its six numbers; a name is a shorthand for a set of them
//! and has no standing the numbers lack (ADR-0029). Nothing here fills a
//! frame on its own — `ChannelLayout::seal` is the call that does.

pub use chdef_core::Crc;

/// What a recipe computes over the bytes it covers.
///
/// Further derivations may appear — a checksum that is not a CRC is not
/// expressible here today — so matches need a catch-all arm. A recipe
/// chdef cannot compute is [`Unknown`](Derivation::Unknown) rather than
/// unreadable: its coverage is still known, which is what
/// [`ChannelLayout::covered_bytes`] hands to a caller computing its own.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Derivation {
    /// A cyclic redundancy check with these parameters.
    Crc(Crc),
    /// Named in the file, not implemented here. Named as written, so a
    /// caller can dispatch on it.
    Unknown(String),
}

/// How a `derived` channel is computed, and over which channels.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedRecipe {
    /// What is computed over the covered bytes.
    pub derivation: Derivation,
    /// The channel numbers it covers, as inclusive spans in the order
    /// they were written. Never empty: a recipe without a range is not a
    /// recipe (`docs/spec/format.md` §6).
    pub spans: Vec<(u32, u32)>,
}

/// The catalogued variants chdef ships, as `(name, parameters)`.
///
/// A name is a shorthand for the numbers beside it. A device whose CRC is
/// in no catalogue writes the numbers instead (ADR-0029).
const CATALOGUE: &[(&str, Crc)] = &[
    (
        "crc16/x25",
        Crc {
            width: 16,
            poly: 0x1021,
            init: 0xFFFF,
            refin: true,
            refout: true,
            xorout: 0xFFFF,
        },
    ),
    (
        "crc16/ibm-3740",
        Crc {
            width: 16,
            poly: 0x1021,
            init: 0xFFFF,
            refin: false,
            refout: false,
            xorout: 0x0000,
        },
    ),
    (
        "crc16/kermit",
        Crc {
            width: 16,
            poly: 0x1021,
            init: 0x0000,
            refin: true,
            refout: true,
            xorout: 0x0000,
        },
    ),
    (
        "crc16/xmodem",
        Crc {
            width: 16,
            poly: 0x1021,
            init: 0x0000,
            refin: false,
            refout: false,
            xorout: 0x0000,
        },
    ),
    (
        "crc8/smbus",
        Crc {
            width: 8,
            poly: 0x07,
            init: 0x00,
            refin: false,
            refout: false,
            xorout: 0x00,
        },
    ),
    (
        "crc32/iso-hdlc",
        Crc {
            width: 32,
            poly: 0x04C1_1DB7,
            init: 0xFFFF_FFFF,
            refin: true,
            refout: true,
            xorout: 0xFFFF_FFFF,
        },
    ),
];

impl DerivedRecipe {
    /// Every recipe name this chdef knows, in catalogue order.
    ///
    /// The set can grow, so a caller can ask what is in it today
    /// (ADR-0026).
    pub fn all() -> Vec<&'static str> {
        CATALOGUE.iter().map(|(name, _)| *name).collect()
    }

    /// The parameters a catalogued name stands for.
    pub fn named(name: &str) -> Option<Crc> {
        let name = name.trim().to_lowercase();
        CATALOGUE
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, crc)| *crc)
    }

    /// Read a `derived` cell (`docs/spec/format.md` §6): a recipe, then
    /// the channels it covers. `None` when the **cell** cannot be read —
    /// no range, a malformed span, malformed parameters. An algorithm
    /// this chdef does not implement is read, as
    /// [`Derivation::Unknown`], because its coverage is still usable.
    pub fn parse(cell: &str) -> Option<DerivedRecipe> {
        let cell = cell.trim();
        let (head, spans) = cell.rsplit_once(char::is_whitespace)?;
        let spans = parse_spans(spans)?;

        let head = head.trim();
        if head.is_empty() {
            return None;
        }
        let derivation = match DerivedRecipe::named(head) {
            Some(crc) => Derivation::Crc(crc),
            None => match parse_parameters(head) {
                Some(crc) => Derivation::Crc(crc),
                // The syntax held and the coverage is known; only the
                // algorithm is one this chdef does not implement.
                None => Derivation::Unknown(head.to_string()),
            },
        };
        Some(DerivedRecipe { derivation, spans })
    }

    /// Whether this recipe covers `number`.
    pub fn covers(&self, number: u32) -> bool {
        self.spans
            .iter()
            .any(|(low, high)| (*low..=*high).contains(&number))
    }
}

impl std::fmt::Display for DerivedRecipe {
    /// The cell form (`docs/spec/format.md` §6), canonical: a catalogued
    /// name where the parameters are one, the parameters otherwise.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.derivation {
            Derivation::Crc(crc) => match CATALOGUE.iter().find(|(_, known)| known == crc) {
                Some((name, _)) => write!(f, "{name}")?,
                None => write!(
                    f,
                    "crc{} poly=0x{:X} init=0x{:X} refin={} refout={} xorout=0x{:X}",
                    crc.width,
                    crc.poly,
                    crc.init,
                    u8::from(crc.refin),
                    u8::from(crc.refout),
                    crc.xorout
                )?,
            },
            Derivation::Unknown(name) => write!(f, "{name}")?,
        }
        for (index, (low, high)) in self.spans.iter().enumerate() {
            let separator = if index == 0 { ' ' } else { ',' };
            write!(f, "{separator}{low}..{high}")?;
        }
        Ok(())
    }
}

/// `1..3` or `1..2,3..3`; every span inclusive, in the order written.
fn parse_spans(text: &str) -> Option<Vec<(u32, u32)>> {
    let mut spans = Vec::new();
    for part in text.split(',') {
        let (low, high) = part.trim().split_once("..")?;
        let low: u32 = low.trim().parse().ok()?;
        let high: u32 = high.trim().parse().ok()?;
        if low > high {
            return None;
        }
        spans.push((low, high));
    }
    (!spans.is_empty()).then_some(spans)
}

/// `crc16 poly=0x1021 init=0xFFFF refin=1 refout=1 xorout=0xFFFF`.
fn parse_parameters(text: &str) -> Option<Crc> {
    let mut words = text.split_whitespace();
    let width: u32 = words.next()?.strip_prefix("crc")?.parse().ok()?;
    if !matches!(width, 8 | 16 | 32 | 64) {
        return None;
    }

    let (mut poly, mut init, mut xorout) = (None, None, None);
    let (mut refin, mut refout) = (None, None);
    for word in words {
        let (key, value) = word.split_once('=')?;
        match key {
            "poly" => poly = Some(number(value)?),
            "init" => init = Some(number(value)?),
            "xorout" => xorout = Some(number(value)?),
            "refin" => refin = Some(flag(value)?),
            "refout" => refout = Some(flag(value)?),
            _ => return None,
        }
    }

    Some(Crc {
        width,
        poly: poly?,
        init: init?,
        refin: refin?,
        refout: refout?,
        xorout: xorout?,
    })
}

fn number(text: &str) -> Option<u64> {
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

fn flag(text: &str) -> Option<bool> {
    match text {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalogued_variant_matches_its_published_check_value() {
        // The self-test each catalogue prints. A parameter mistyped here
        // shows up as a check value that is not the published one.
        let published = [
            ("crc16/x25", 0x906E),
            ("crc16/ibm-3740", 0x29B1),
            ("crc16/kermit", 0x2189),
            ("crc16/xmodem", 0x31C3),
            ("crc8/smbus", 0xF4),
            ("crc32/iso-hdlc", 0xCBF4_3926),
        ];
        assert_eq!(published.len(), CATALOGUE.len(), "a variant is unchecked");
        for (name, check) in published {
            let crc = DerivedRecipe::named(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(crc.check(), check, "{name}");
        }
    }

    #[test]
    fn an_algorithm_this_chdef_does_not_implement_keeps_its_coverage() {
        // The escape hatch: a device with a checksum chdef never heard of
        // still says which bytes it covers, so a caller can compute its
        // own over exactly those.
        let recipe = DerivedRecipe::parse("fletcher16 1..7").unwrap();
        assert_eq!(
            recipe.derivation,
            Derivation::Unknown("fletcher16".to_string())
        );
        assert_eq!(recipe.spans, vec![(1, 7)]);
    }

    #[test]
    fn a_name_and_its_numbers_are_the_same_recipe() {
        let named = DerivedRecipe::parse("crc16/x25 1..3").unwrap();
        let spelled = DerivedRecipe::parse(
            "crc16 poly=0x1021 init=0xFFFF refin=1 refout=1 xorout=0xFFFF 1..3",
        )
        .unwrap();
        assert_eq!(named, spelled);
    }

    #[test]
    fn a_recipe_needs_a_range() {
        assert_eq!(DerivedRecipe::parse("crc16/x25"), None);
        assert_eq!(DerivedRecipe::parse("crc16/x25 "), None);
    }

    #[test]
    fn spans_are_read_in_the_order_written() {
        let recipe = DerivedRecipe::parse("crc16/x25 5..7,1..2").unwrap();
        assert_eq!(recipe.spans, vec![(5, 7), (1, 2)]);
        assert!(recipe.covers(6) && recipe.covers(1) && !recipe.covers(3));
    }

    #[test]
    fn a_cell_this_chdef_cannot_read_is_no_recipe() {
        // A cell is unreadable when its *syntax* fails, never merely
        // because the algorithm is unimplemented.
        for cell in [
            "crc16/x25 1..",
            "crc16/x25 3..1",
            "crc16/x25 a..b",
            "",
            "1..1",
        ] {
            assert_eq!(DerivedRecipe::parse(cell), None, "{cell}");
        }
    }
}
