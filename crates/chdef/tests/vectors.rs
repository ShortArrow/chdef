//! Runs every golden vector set in `vectors/` against this implementation.
//! The same files are the contract other languages verify themselves
//! against (`docs/spec/interchange.md` §3).

use std::path::{Path, PathBuf};

use chdef::{build_layout, ChannelLayout, Value};

fn vector_sets() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors");
    let mut sets: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    sets.sort();
    assert!(!sets.is_empty(), "no vector set in {}", root.display());
    sets
}

fn layout_of(dir: &Path) -> ChannelLayout {
    let channels = chdef::load_ch_csv(dir.join("ch.csv")).unwrap();
    let bitfields = chdef::load_bf_csv(dir.join("bf.csv")).unwrap();
    assert!(
        channels.issues.is_empty() && bitfields.issues.is_empty(),
        "{}: the definitions of a vector set must load without Issues, got {:?} {:?}",
        dir.display(),
        channels.issues,
        bitfields.issues
    );
    let layout = build_layout(channels.value, bitfields.value);
    assert!(
        layout.issues.is_empty(),
        "{}: {:?}",
        dir.display(),
        layout.issues
    );
    layout.value
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    assert!(hex.len() % 2 == 0, "odd hex string {hex}");
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse_values(field: &str) -> Vec<(u32, Value)> {
    if field == "-" {
        return Vec::new();
    }
    field
        .split(';')
        .map(|pair| {
            let (number, value) = pair.split_once('=').expect("n=value");
            (
                number.parse().expect("channel number"),
                Value::parse(value).expect("value notation"),
            )
        })
        .collect()
}

fn check_encode(layout: &ChannelLayout, values: &str, expected_hex: &str, at: &str) {
    let encoded = layout.encode(&parse_values(values));
    assert!(encoded.issues.is_empty(), "{at}: {:?}", encoded.issues);
    assert_eq!(bytes_to_hex(&encoded.value), expected_hex, "{at}");
}

fn check_decode(layout: &ChannelLayout, frame_hex: &str, expected: &str, at: &str) {
    let frame = hex_to_bytes(frame_hex);
    let decoded = layout.decode(&frame);
    let expected: Vec<&str> = expected.split(';').collect();
    assert_eq!(decoded.len(), expected.len(), "{at}: channel count");

    for (reading, want) in decoded.iter().zip(expected) {
        let (number, rest) = want.split_once('=').expect("n=raw/value");
        let (raw, value) = rest.split_once('/').expect("raw/value");
        assert_eq!(reading.channel.number.to_string(), number, "{at}: number");
        assert_eq!(reading.raw.to_string(), raw, "{at}: channel {number} raw");
        let want: f64 = value.parse().unwrap();
        assert!(
            (reading.value - want).abs() < 1e-9,
            "{at}: channel {number} value {} != {want}",
            reading.value
        );
    }
}

fn check_layout(layout: &ChannelLayout, total: &str, positions: &str, at: &str) {
    assert_eq!(layout.total_bytes().to_string(), total, "{at}: total_bytes");
    let expected: Vec<&str> = positions.split(';').collect();
    let actual: Vec<String> = layout
        .positions()
        .map(|(offset, ch)| format!("{}:{offset}:{}", ch.number, ch.byte_count))
        .collect();
    assert_eq!(actual, expected, "{at}: positions");
}

#[test]
fn every_golden_vector_holds() {
    for dir in vector_sets() {
        let layout = layout_of(&dir);
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(dir.join("vectors.txt")).unwrap();

        let (mut encodes, mut decodes, mut layouts) = (0, 0, 0);
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let at = format!("{name}/vectors.txt:{}", index + 1);
            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields.as_slice() {
                ["E", values, hex] => {
                    check_encode(&layout, values, hex, &at);
                    encodes += 1;
                }
                ["D", hex, expected] => {
                    check_decode(&layout, hex, expected, &at);
                    decodes += 1;
                }
                ["L", total, positions] => {
                    check_layout(&layout, total, positions, &at);
                    layouts += 1;
                }
                _ => panic!("{at}: unreadable vector line {line:?}"),
            }
        }

        assert!(
            encodes > 0 && decodes > 0 && layouts > 0,
            "{name}: a vector set needs at least one E, D and L line"
        );
    }
}
