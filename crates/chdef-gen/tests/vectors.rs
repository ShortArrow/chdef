//! The golden vectors of `docs/spec/interchange.md` §3, read through the
//! table a device would hold.
//!
//! The same files certify the host. Here they certify that nothing is lost
//! on the way into the constant: a channel sits where the `L` line says,
//! a frame reads back the raw values the `D` lines state, and an
//! all-defaults frame is the one the `E -` lines spell out. A set whose
//! own definitions carry findings is a set no device may be given at all.

use std::path::{Path, PathBuf};

use chdef::ColumnVocabulary;
use chdef_core::{Endian, Layout};
use chdef_gen::{model, Model, Refusal};

fn vector_sets() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .join("chdef/vectors");
    let mut sets: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    sets.sort();
    assert!(!sets.is_empty(), "no vector set in {}", root.display());
    sets
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    assert!(hex.len() % 2 == 0, "odd hex string {hex}");
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `<total> <n:at:bytes;...>` against the slots the table carries.
fn check_layout(layout: &Layout<'_>, total: &str, positions: &str, at: &str) {
    assert_eq!(layout.total.to_string(), total, "{at}: total");
    let expected: Vec<&str> = positions.split(';').collect();
    let actual: Vec<String> = layout
        .slots
        .iter()
        .map(|slot| format!("{}:{}:{}", slot.number, slot.at, slot.width()))
        .collect();
    assert_eq!(actual, expected, "{at}: positions");
}

/// `<hex> <n=raw/value;...>`: the raw values of the channels the frame
/// reaches, and nothing of the ones it does not.
fn check_decode(layout: &Layout<'_>, frame_hex: &str, expected: &str, at: &str) {
    let frame = hex_to_bytes(frame_hex);
    let mut listed = Vec::new();
    for want in expected.split(';') {
        let (number, rest) = want.split_once('=').expect("n=raw/value");
        let (raw, _) = rest.split_once('/').expect("raw/value");
        let number: u32 = number.parse().expect("channel number");
        let raw: u64 = raw.parse().expect("raw value");
        assert_eq!(
            layout.read(&frame, number),
            Some(raw),
            "{at}: channel {number}"
        );
        listed.push(number);
    }
    for slot in layout.slots {
        if !listed.contains(&slot.number) {
            assert_eq!(
                layout.read(&frame, slot.number),
                None,
                "{at}: channel {} is past the end of the frame",
                slot.number
            );
        }
    }
}

/// `E - <hex>`: the frame every channel's default makes
/// (`docs/spec/conversion.md` §4).
fn check_defaults(layout: &Layout<'_>, expected: &str, at: &str) {
    let mut frame = vec![0u8; layout.total as usize];
    assert!(layout.fill_defaults(&mut frame), "{at}: fill_defaults");
    assert_eq!(bytes_to_hex(&frame), expected, "{at}: defaults");
}

fn built(directory: &Path, endian: Endian) -> Result<Model, Refusal> {
    let ch = std::fs::read(directory.join("ch.csv")).expect("ch.csv");
    let bf = std::fs::read(directory.join("bf.csv")).unwrap_or_default();
    model(&ch, &bf, endian, &ColumnVocabulary::new())
}

#[test]
fn every_golden_vector_reaches_the_table() {
    for directory in vector_sets() {
        let name = directory
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .to_string();
        let text = std::fs::read_to_string(directory.join("vectors.txt")).expect("vectors.txt");
        let lines: Vec<(usize, Vec<&str>)> = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
            .map(|(number, line)| (number, line.split_whitespace().collect()))
            .collect();

        // A set that declares findings of its own describes a definition
        // no device may be given, whatever else it says.
        if lines.iter().any(|(_, fields)| fields.first() == Some(&"P")) {
            match built(&directory, Endian::Little) {
                Err(Refusal::Issues(issues)) => assert!(!issues.is_empty(), "{name}"),
                other => panic!("{name}: a definition with findings became {other:?}"),
            }
            println!("{name}: refused, as its P lines declare");
            continue;
        }

        let mut model =
            built(&directory, Endian::Little).unwrap_or_else(|refusal| panic!("{name}: {refusal}"));
        let (mut layouts, mut decodes, mut skipped) = (0, 0, 0);

        for (number, fields) in &lines {
            let at = format!("{name}/vectors.txt:{number}");
            match fields.as_slice() {
                ["B", order] => {
                    let endian = match *order {
                        "little" => Endian::Little,
                        "big" => Endian::Big,
                        other => panic!("{at}: unknown byte order {other:?}"),
                    };
                    model = built(&directory, endian)
                        .unwrap_or_else(|refusal| panic!("{name}: {refusal}"));
                }
                ["L", total, positions] => {
                    check_layout(&model.layout().as_layout(), total, positions, &at);
                    layouts += 1;
                }
                ["D", hex, expected] => {
                    check_decode(&model.layout().as_layout(), hex, expected, &at);
                    decodes += 1;
                }
                ["E", "-", hex] => check_defaults(&model.layout().as_layout(), hex, &at),
                // Physical values and named bits are the host's; the table
                // carries neither.
                ["E", ..] | ["F", ..] => skipped += 1,
                _ => panic!("{at}: unreadable vector line {fields:?}"),
            }
        }

        assert!(
            layouts > 0 && decodes > 0,
            "{name}: {layouts} layouts and {decodes} frames checked"
        );
        println!("{name}: {layouts} layouts, {decodes} frames, {skipped} lines the table has no answer for");
    }
}
