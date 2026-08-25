//! The golden vectors of `docs/spec/interchange.md` §3, run through the C
//! ABI instead of the crate's Rust API.
//!
//! This is the point of the ABI (ADR-0021): an ABI that could drift from
//! the crate would reproduce the divergence it exists to remove, so the
//! same contract that certifies a C# implementation certifies the boundary
//! the C# implementation will call.

use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::ptr;

use chdef_capi::*;

fn vector_sets() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("chdef")
        .join("vectors");
    let mut sets: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    sets.sort();
    assert!(!sets.is_empty(), "no vector set in {}", root.display());
    sets
}

struct Layout(*mut ChdefLayout, *mut ChdefIssues);

impl Layout {
    fn of(dir: &Path) -> Layout {
        let ch = std::fs::read_to_string(dir.join("ch.csv")).unwrap();
        let bf = std::fs::read_to_string(dir.join("bf.csv")).unwrap();
        let mut layout = ptr::null_mut();
        let mut issues = ptr::null_mut();
        let mut err = [0u8; 256];
        let status = unsafe {
            chdef_layout_parse(
                ch.as_ptr(),
                ch.len(),
                bf.as_ptr(),
                bf.len(),
                &mut layout,
                &mut issues,
                err.as_mut_ptr() as *mut c_char,
                err.len(),
            )
        };
        assert_eq!(status, CHDEF_OK, "{}: parse failed", dir.display());
        Layout(layout, issues)
    }

    /// Every diagnostic the parse produced, as `code:row` — the shape the
    /// `P` lines are written in.
    fn issues(&self) -> Vec<String> {
        let count = unsafe { chdef_issue_count(self.1) } as usize;
        let mut listed: Vec<String> = (0..count)
            .map(|index| {
                let mut issue = ChdefIssue::default();
                assert_eq!(
                    unsafe { chdef_issue_at(self.1, index, &mut issue) },
                    CHDEF_OK
                );
                let code = text(|buf, cap| unsafe {
                    chdef_issue_text(self.1, index, CHDEF_ISSUE_CODE, buf, cap)
                });
                match issue.row {
                    -1 => format!("{code}:-"),
                    row => format!("{code}:{row}"),
                }
            })
            .collect();
        listed.sort();
        listed
    }

    fn set_endian(&self, endian: i32) {
        assert_eq!(unsafe { chdef_layout_set_endian(self.0, endian) }, CHDEF_OK);
    }
}

impl Drop for Layout {
    fn drop(&mut self) {
        unsafe {
            chdef_issues_free(self.1);
            chdef_layout_free(self.0);
        }
    }
}

/// Ask a text call for its length, then for its value.
fn text(mut call: impl FnMut(*mut c_char, usize) -> usize) -> String {
    let needed = call(ptr::null_mut(), 0);
    let mut buf = vec![0u8; needed + 1];
    call(buf.as_mut_ptr() as *mut c_char, buf.len());
    buf.truncate(needed);
    String::from_utf8(buf).unwrap()
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    assert!(hex.len() % 2 == 0, "odd hex string {hex}");
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// The bits a frame yields, keyed `channel:bit`.
fn check_bits(layout: &Layout, frame_hex: &str, expected: &str, at: &str) {
    let frame = hex_to_bytes(frame_hex);
    let total = unsafe { chdef_layout_bit_total(layout.0) } as usize;
    let mut out = vec![ChdefBitReading::default(); total];
    let mut count = 0usize;
    assert_eq!(
        unsafe {
            chdef_decode_bits(
                layout.0,
                frame.as_ptr(),
                frame.len(),
                out.as_mut_ptr(),
                out.len(),
                &mut count,
            )
        },
        CHDEF_OK,
        "{at}"
    );

    for want in expected.split(';') {
        let (position, value) = want.split_once('=').expect("n:bit=value");
        let (number, bit) = position.split_once(':').expect("n:bit");
        let number: u32 = number.parse().expect("channel number");
        let bit: u32 = bit.parse().expect("bit number");
        let value: i32 = value.parse().expect("0 or 1");

        let read = out[..count]
            .iter()
            .find(|b| b.channel == number && b.bit == bit)
            .unwrap_or_else(|| panic!("{at}: bit {bit} of channel {number} is not in the frame"));
        assert_eq!(read.value, value, "{at}: bit {bit} of channel {number}");
    }
}

/// The `P` lines name Issues per source file; the ABI merges the three
/// lists into the one its parse returns, so what is contracted here is
/// their union.
fn declared(field: &str) -> Vec<String> {
    if field == "-" {
        return Vec::new();
    }
    field.split(';').map(str::to_string).collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn values_of(field: &str) -> Vec<ChdefValue> {
    if field == "-" {
        return Vec::new();
    }
    field
        .split(';')
        .map(|pair| {
            let (number, value) = pair.split_once('=').expect("n=value");
            let channel = number.parse().expect("channel number");
            match value
                .strip_prefix("0x")
                .or_else(|| value.strip_prefix("0X"))
            {
                Some(hex) => ChdefValue::raw(channel, u64::from_str_radix(hex, 16).unwrap()),
                None => ChdefValue::physical(channel, value.parse().expect("physical value")),
            }
        })
        .collect()
}

fn check_encode(
    layout: &Layout,
    values: &str,
    expected_hex: &str,
    expected_issues: Option<&str>,
    at: &str,
) {
    let given = values_of(values);
    let mut frame = vec![0u8; unsafe { chdef_layout_total_bytes(layout.0) } as usize];
    let mut written = 0usize;
    let mut issues = ptr::null_mut();

    let status = unsafe {
        chdef_encode(
            layout.0,
            given.as_ptr(),
            given.len(),
            frame.as_mut_ptr(),
            frame.len(),
            &mut written,
            &mut issues,
        )
    };

    assert_eq!(status, CHDEF_OK, "{at}");
    frame.truncate(written);
    assert_eq!(bytes_to_hex(&frame), expected_hex, "{at}");

    let found: Vec<String> = (0..unsafe { chdef_issue_count(issues) } as usize)
        .map(|index| {
            let mut issue = ChdefIssue::default();
            assert_eq!(
                unsafe { chdef_issue_at(issues, index, &mut issue) },
                CHDEF_OK
            );
            let code = text(|buf, cap| unsafe {
                chdef_issue_text(issues, index, CHDEF_ISSUE_CODE, buf, cap)
            });
            match issue.channel {
                -1 => format!("{code}:-"),
                channel => format!("{code}:{channel}"),
            }
        })
        .collect();
    unsafe { chdef_issues_free(issues) };

    let expected = match expected_issues {
        None | Some("-") => Vec::new(),
        Some(list) => list.split(';').map(str::to_string).collect::<Vec<_>>(),
    };
    assert_eq!(found, expected, "{at}: the Issues of the encode");
}

fn check_decode(layout: &Layout, frame_hex: &str, expected: &str, at: &str) {
    let frame = hex_to_bytes(frame_hex);
    let expected: Vec<&str> = expected.split(';').collect();
    let mut readings = vec![ChdefReading::default(); expected.len().max(1)];
    let mut count = 0usize;

    let status = unsafe {
        chdef_decode(
            layout.0,
            frame.as_ptr(),
            frame.len(),
            readings.as_mut_ptr(),
            readings.len(),
            &mut count,
        )
    };

    assert_eq!(status, CHDEF_OK, "{at}");
    assert_eq!(count, expected.len(), "{at}: channel count");
    for (reading, want) in readings.iter().zip(expected) {
        let (number, rest) = want.split_once('=').expect("n=raw/value");
        let (raw, value) = rest.split_once('/').expect("raw/value");
        assert_eq!(reading.channel.to_string(), number, "{at}: number");
        assert_eq!(reading.raw.to_string(), raw, "{at}: channel {number} raw");
        let want: f64 = value.parse().unwrap();
        assert!(
            (reading.value - want).abs() <= 1e-9 * want.abs().max(1.0),
            "{at}: channel {number} value {} != {want}",
            reading.value
        );
    }
}

fn check_layout(layout: &Layout, total: &str, positions: &str, at: &str) {
    assert_eq!(
        unsafe { chdef_layout_total_bytes(layout.0) }.to_string(),
        total,
        "{at}: total_bytes"
    );
    let expected: Vec<&str> = positions.split(';').collect();
    let count = unsafe { chdef_layout_channel_count(layout.0) } as usize;
    assert_eq!(count, expected.len(), "{at}: channel count");

    for (index, want) in expected.iter().enumerate() {
        let mut ch = ChdefChannel::default();
        assert_eq!(
            unsafe { chdef_layout_channel_at(layout.0, index, &mut ch) },
            CHDEF_OK,
            "{at}"
        );
        assert_eq!(
            format!("{}:{}:{}", ch.number, ch.at, ch.bytes),
            *want,
            "{at}: position {index}"
        );
    }
}

#[test]
fn every_golden_vector_holds_through_the_abi() {
    for dir in vector_sets() {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(dir.join("vectors.txt")).unwrap();
        let layout = Layout::of(&dir);
        let mut checked = 0;
        let mut expected: Vec<String> = Vec::new();
        let mut sources = 0;

        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let at = format!("{name}/vectors.txt:{} (through the ABI)", index + 1);
            match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["B", "little"] => layout.set_endian(CHDEF_LITTLE),
                ["B", "big"] => layout.set_endian(CHDEF_BIG),
                ["E", values, hex, issues] => {
                    check_encode(&layout, values, hex, Some(issues), &at);
                    checked += 1;
                }
                ["E", values, hex] => {
                    check_encode(&layout, values, hex, None, &at);
                    checked += 1;
                }
                ["D", hex, expected] => {
                    check_decode(&layout, hex, expected, &at);
                    checked += 1;
                }
                ["L", total, positions] => {
                    check_layout(&layout, total, positions, &at);
                    checked += 1;
                }
                ["F", hex, bits] => {
                    check_bits(&layout, hex, bits, &at);
                    checked += 1;
                }
                ["P", _source, issues] => {
                    expected.extend(declared(issues));
                    sources += 1;
                }
                _ => panic!("{at}: unreadable vector line {line:?}"),
            }
        }

        if sources > 0 {
            expected.sort();
            assert_eq!(
                layout.issues(),
                expected,
                "{name}: the Issues of the definitions, through the ABI"
            );
        }

        assert!(checked > 0, "{name}: nothing was checked through the ABI");
    }
}
