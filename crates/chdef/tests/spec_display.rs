//! Tests for the `format` column and what it selects, derived from
//! `docs/spec/format.md` §3 and `docs/spec/conversion.md` §7.
//!
//! conversion.md §7: "`format` does not affect chdef's conversion (`value`
//! is returned even for `HEX`). The consumer shows the raw value when the
//! format is `HEX`." What the column selects is therefore which value is
//! shown, not the base it is printed in.

use chdef::*;

// format.md §3: the cell says `DEC` / `HEX`, case-insensitively.
#[test]
fn the_column_is_still_spelled_dec_and_hex() {
    assert_eq!(ValueDisplay::parse("DEC"), Some(ValueDisplay::Physical));
    assert_eq!(ValueDisplay::parse("hex"), Some(ValueDisplay::Raw));
    assert_eq!(ValueDisplay::parse(" Hex "), Some(ValueDisplay::Raw));
    assert_eq!(ValueDisplay::parse("octal"), None);

    assert_eq!(ValueDisplay::Physical.as_str(), "DEC");
    assert_eq!(ValueDisplay::Raw.as_str(), "HEX");
}

// §3: "Empty / unknown → `DEC`."
#[test]
fn an_empty_or_unknown_format_shows_the_physical_value() {
    let parsed = parse_ch_csv("number,format,name\n1,,a\n2,octal,b\n3,HEX,c\n").unwrap();

    assert_eq!(parsed.value[0].format, ValueDisplay::Physical);
    assert_eq!(parsed.value[1].format, ValueDisplay::Physical);
    assert_eq!(parsed.value[2].format, ValueDisplay::Raw);
}

// conversion.md §7: the column picks which value is shown.
#[test]
fn the_column_selects_which_value_is_shown() {
    let mut physical = ChannelDef::new(1, 2, DataType::UI);
    physical.lsb = 0.5;
    let mut raw = physical.clone();
    raw.format = ValueDisplay::Raw;

    assert_eq!(physical.displayed_value(10), Value::Physical(5.0));
    assert_eq!(raw.displayed_value(10), Value::Raw(10));
}

// §7: "`value` is returned even for `HEX`" — selecting the raw value for
// display changes no conversion.
#[test]
fn selecting_the_raw_value_changes_no_conversion() {
    let mut ch = ChannelDef::new(1, 2, DataType::UI);
    ch.lsb = 0.5;
    ch.format = ValueDisplay::Raw;

    assert_eq!(ch.raw_to_value_u64(10), 5.0);
    assert_eq!(ch.value_to_raw(5.0), Some(10));
}

// The default rendering a consumer may take or replace.
#[test]
fn render_shows_the_selected_value_with_the_unit() {
    let mut ch = ChannelDef::new(1, 2, DataType::SI);
    ch.lsb = 0.1;
    ch.unit = "degC".into();

    assert_eq!(ch.render(0xFF85), "-12.3 degC");

    ch.format = ValueDisplay::Raw;
    assert_eq!(ch.render(0xFF85), "0xFF85");
}

// A raw rendering is as wide as the channel, so the digits line up in a
// column of readings.
#[test]
fn a_raw_rendering_is_as_wide_as_the_channel() {
    let mut narrow = ChannelDef::new(1, 1, DataType::UI);
    narrow.format = ValueDisplay::Raw;
    let mut wide = ChannelDef::new(2, 4, DataType::UI);
    wide.format = ValueDisplay::Raw;

    assert_eq!(narrow.render(0x07), "0x07");
    assert_eq!(wide.render(0x07), "0x00000007");
}

// A channel with no unit renders the bare number.
#[test]
fn a_channel_without_a_unit_renders_the_bare_number() {
    let ch = ChannelDef::new(1, 2, DataType::UI);

    assert_eq!(ch.render(42), "42");
}

// format.md §3: "`HEX` with `lsb` ≠ 1 → Issue". The code names what is
// wrong — the channel shows a raw value that is not the physical quantity.
#[test]
fn showing_a_raw_value_under_a_scaling_lsb_is_reported() {
    let parsed = parse_ch_csv("number,lsb,format,name\n1,0.1,HEX,a\n2,1,HEX,b\n").unwrap();

    let codes: Vec<IssueCode> = parsed.issues.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::RawDisplayWithLsb]);
    assert_eq!(
        IssueCode::RawDisplayWithLsb.as_str(),
        "raw_display_with_lsb"
    );
}

// The Table writes the column back in the spelling the file uses.
#[test]
fn the_table_writes_the_column_back_as_dec_or_hex() {
    let mut table = ChTable::new();
    let mut shown_raw = ChannelDef::new(1, 2, DataType::UI);
    shown_raw.format = ValueDisplay::Raw;

    table.insert_channel(0, &shown_raw);
    table.insert_channel(1, &ChannelDef::new(2, 2, DataType::UI));

    let csv = table.to_csv();
    assert!(csv.lines().nth(1).unwrap().contains("HEX"));
    assert!(csv.lines().nth(2).unwrap().contains("DEC"));
}
