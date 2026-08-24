//! Tests derived from the wording of `docs/spec/conversion.md` §1 / §2 and
//! `docs/spec/layout.md` §6, over **every** width the specification calls
//! legal — not over the widths the implementation happens to favour.
//!
//! layout.md §6: "The width is `bytes` (1–8). `type` carries no width, only
//! the interpretation (`UI` / `SI` / `BF`)."

use chdef::*;

/// Every width `docs/spec/format.md` §3 accepts for the `bytes` column.
const LEGAL_WIDTHS: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

fn channel(width: usize, data_type: DataType) -> ChannelDef {
    ChannelDef::new(1, width, data_type)
}

fn layout_of(ch: ChannelDef, endian: Endian) -> ChannelLayout {
    let mut layout = build_layout(vec![ch], vec![]).value;
    layout.endian = endian;
    layout
}

/// The all-ones pattern of a width, and the width's bit count.
fn all_ones(width: usize) -> (u64, u32) {
    let bits = (width * 8) as u32;
    let mask = if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    (mask, bits)
}

/// A pattern whose every byte differs, so dropping any byte changes it.
fn distinct_bytes(width: usize) -> u64 {
    (0..width).fold(0u64, |raw, i| raw | ((i as u64 + 1) << (8 * i)))
}

// §1: "`raw_signed`: the raw value of width `bytes × 8` bits ... read as
// unsigned for `UI` / `BF`."
#[test]
fn an_unsigned_channel_reads_its_whole_width() {
    for width in LEGAL_WIDTHS {
        for data_type in [DataType::UI, DataType::BF] {
            let ch = channel(width, data_type);
            let raw = distinct_bytes(width);

            assert_eq!(
                ch.raw_to_value_u64(raw),
                raw as f64,
                "{data_type:?} of {width} bytes must read all {width} bytes of {raw:#X}"
            );
        }
    }
}

// §1: "sign-extended as two's complement for `SI`"; layout.md §6: "`SI` is
// sign-extended at its width."
#[test]
fn a_signed_channel_sign_extends_from_its_own_width() {
    for width in LEGAL_WIDTHS {
        let ch = channel(width, DataType::SI);
        let (ones, bits) = all_ones(width);

        assert_eq!(
            ch.raw_to_value_u64(ones),
            -1.0,
            "all ones of {width} bytes is -1"
        );

        let most_negative = 1u64 << (bits - 1);
        assert_eq!(
            ch.raw_to_value_u64(most_negative),
            -(2f64.powi(bits as i32 - 1)),
            "the top bit alone of {width} bytes is the most negative value"
        );

        let most_positive = most_negative - 1;
        assert_eq!(
            ch.raw_to_value_u64(most_positive),
            2f64.powi(bits as i32 - 1) - 1.0,
            "one below the top bit of {width} bytes is the most positive value"
        );
    }
}

// §1: "Bits beyond the width are ignored."
#[test]
fn bits_beyond_the_width_are_ignored() {
    for width in LEGAL_WIDTHS.into_iter().take(7) {
        let ch = channel(width, DataType::UI);
        let (ones, bits) = all_ones(width);

        assert_eq!(
            ch.raw_to_value_u64(ones | (1u64 << bits)),
            ones as f64,
            "a bit above the {width}-byte width must not reach the value"
        );
    }
}

// §1's formula in full: `value = raw_signed × lsb + offset`.
#[test]
fn lsb_and_offset_apply_at_every_width() {
    for width in LEGAL_WIDTHS {
        let mut ch = channel(width, DataType::UI);
        ch.lsb = 0.5;
        ch.offset = -3.0;

        assert_eq!(
            ch.raw_to_value_u64(10),
            10.0 * 0.5 - 3.0,
            "{width} bytes: raw × lsb + offset"
        );
    }
}

// conversion.md §6 hands back both readings of one channel; with `lsb` 1 and
// `offset` 0 the physical value is the raw value, at every width.
#[test]
fn the_two_readings_of_a_decoded_channel_agree_at_every_width() {
    for width in LEGAL_WIDTHS {
        for endian in [Endian::Little, Endian::Big] {
            let layout = layout_of(channel(width, DataType::UI), endian);
            let frame = layout
                .encode(&[(1, Value::Raw(distinct_bytes(width)))])
                .value;

            let decoded = layout.decode(&frame);

            assert_eq!(
                decoded[0].value, decoded[0].raw as f64,
                "{width} bytes, {endian:?}: raw {:#X} and value {} disagree",
                decoded[0].raw, decoded[0].value
            );
        }
    }
}

// §5 then §6: what encode wrote, decode reads back, at every width.
#[test]
fn encode_and_decode_round_trip_at_every_width_and_byte_order() {
    for width in LEGAL_WIDTHS {
        for endian in [Endian::Little, Endian::Big] {
            for data_type in [DataType::UI, DataType::SI, DataType::BF] {
                let layout = layout_of(channel(width, data_type), endian);
                let raw = distinct_bytes(width);

                let frame = layout.encode(&[(1, Value::Raw(raw))]).value;
                assert_eq!(frame.len(), width, "{width} bytes, {data_type:?}");

                assert_eq!(
                    layout.decode(&frame)[0].raw,
                    raw,
                    "{width} bytes, {endian:?}, {data_type:?}: raw did not survive"
                );
            }
        }
    }
}

// A physical value survives the same round trip.
#[test]
fn a_physical_value_survives_the_round_trip_at_every_width() {
    for width in LEGAL_WIDTHS {
        let mut ch = channel(width, DataType::SI);
        ch.lsb = 0.25;
        ch.offset = -8.0;
        let layout = layout_of(ch, Endian::Big);

        let frame = layout.encode(&[(1, Value::Physical(-4.5))]).value;

        assert_eq!(
            layout.decode(&frame)[0].value,
            -4.5,
            "{width} bytes: the physical value did not survive"
        );
    }
}

// layout.md §2: the byte order is the layout's, at every width.
#[test]
fn the_layout_byte_order_reverses_the_bytes_at_every_width() {
    for width in LEGAL_WIDTHS {
        let little = layout_of(channel(width, DataType::UI), Endian::Little)
            .encode(&[(1, Value::Raw(1))])
            .value;
        let big = layout_of(channel(width, DataType::UI), Endian::Big)
            .encode(&[(1, Value::Raw(1))])
            .value;

        assert_eq!(little.first(), Some(&1u8), "{width} bytes little-endian");
        assert_eq!(big.last(), Some(&1u8), "{width} bytes big-endian");
        assert_eq!(big.iter().rev().copied().collect::<Vec<u8>>(), little);
    }
}

// §2: "`clamp`: `SI` to `[−2^(bits−1), 2^(bits−1) − 1]`, `UI` / `BF` to
// `[0, 2^bits − 1]`."
#[test]
fn physical_to_raw_clamps_to_the_width_at_every_width() {
    for width in LEGAL_WIDTHS {
        let (ones, bits) = all_ones(width);
        let top = 2f64.powi(bits as i32 - 1);

        let unsigned = channel(width, DataType::UI);
        assert_eq!(
            unsigned.value_to_raw(2f64.powi(bits as i32) * 2.0),
            Some(ones),
            "{width} bytes unsigned clamps to all ones"
        );
        assert_eq!(
            unsigned.value_to_raw(-1.0),
            Some(0),
            "{width} bytes unsigned clamps up to zero"
        );

        let signed = channel(width, DataType::SI);
        assert_eq!(
            signed.value_to_raw(top * 2.0),
            Some(ones >> 1),
            "{width} bytes signed clamps to its most positive value"
        );
        assert_eq!(
            signed.value_to_raw(-top * 2.0),
            Some(1u64 << (bits - 1)),
            "{width} bytes signed clamps to its most negative value"
        );
    }
}

// §2: "`round` rounds half away from zero (0.5 → 1, −0.5 → −1, 2.5 → 3)."
#[test]
fn rounding_is_half_away_from_zero_at_every_width() {
    for width in LEGAL_WIDTHS {
        let unsigned = channel(width, DataType::UI);
        assert_eq!(
            unsigned.value_to_raw(0.5),
            Some(1),
            "{width} bytes: 0.5 → 1"
        );
        assert_eq!(
            unsigned.value_to_raw(2.5),
            Some(3),
            "{width} bytes: 2.5 → 3"
        );

        let signed = channel(width, DataType::SI);
        let (ones, _) = all_ones(width);
        assert_eq!(
            signed.value_to_raw(-0.5),
            Some(ones),
            "{width} bytes: −0.5 → −1"
        );
    }
}

// layout.md §6: the width is `bytes`; `type` carries none of it. The same
// interpretation at two widths must differ only by the width.
#[test]
fn the_type_column_carries_no_width() {
    let narrow = channel(2, DataType::UI);
    let wide = channel(6, DataType::UI);

    assert_eq!(narrow.bits(), 16);
    assert_eq!(wide.bits(), 48);
    assert_eq!(narrow.raw_to_value_u64(0xFFFF), 65535.0);
    assert_eq!(
        wide.raw_to_value_u64(0xFFFF_FFFF_FFFF),
        281_474_976_710_655.0
    );
}

// A width outside 1–8 never reaches the arithmetic: format.md §3 clamps the
// column to 1–8, and a definition built in code is treated the same way
// rather than panicking.
#[test]
fn a_width_outside_the_legal_range_is_treated_as_the_nearest_legal_one() {
    assert_eq!(channel(0, DataType::SI).bits(), 8);
    assert_eq!(channel(99, DataType::UI).bits(), 64);
    assert_eq!(channel(0, DataType::SI).raw_to_value_u64(0xFF), -1.0);

    let layout = layout_of(channel(0, DataType::UI), Endian::Little);
    assert_eq!(layout.total_bytes(), 1);
    assert_eq!(layout.encode(&[(1, Value::Raw(0xAB))]).value, vec![0xAB]);
}
