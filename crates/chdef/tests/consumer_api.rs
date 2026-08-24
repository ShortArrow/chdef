//! The pieces consumers reported rebuilding on top of chdef: a layout that
//! carries the capacity it is measured against, the bits of a decoded `BF`
//! channel, and the grid a table editor reads.

use chdef::*;

fn bf_layout() -> ChannelLayout {
    let mut flags = ChannelDef::new(1, 2, DataType::BF);
    flags.name = "flags".into();
    flags.default_value = Some(0x0005);

    let mut alive = BitFieldDef::new(1, 0);
    alive.name = "alive".into();
    let mut fault = BitFieldDef::new(1, 2);
    fault.name = "fault".into();
    fault.default_value = Some(0);
    let mut ready = BitFieldDef::new(1, 7);
    ready.name = "ready".into();
    ready.default_value = Some(1);

    build_layout(
        vec![flags, ChannelDef::new(2, 1, DataType::UI)],
        vec![alive, fault, ready],
    )
    .value
}

// layout.md §5: a consumer that has a capacity asks the layout about it.
// Carrying it beside the layout is what the consumer had to do instead.
#[test]
fn a_layout_carries_the_capacity_it_is_measured_against() {
    let layout = bf_layout().with_capacity(8);

    assert_eq!(layout.capacity, Some(8));
    assert!(layout.check_capacity().is_none(), "3 bytes fit in 8");

    let tight = bf_layout().with_capacity(2);
    let issue = tight.check_capacity().expect("3 bytes do not fit in 2");
    assert_eq!(issue.code, IssueCode::LayoutExceedsCapacity);
    assert_eq!(issue.row, None);
}

// Without one there is no check, as §5 has always said.
#[test]
fn a_layout_without_a_capacity_checks_nothing() {
    let layout = bf_layout();

    assert_eq!(layout.capacity, None);
    assert!(layout.check_capacity().is_none());
}

// conversion.md §6: "A BF bit is `(raw >> bit) & 1` of the parent's raw
// value." Walking the named bits of a decoded channel is the reading a
// consumer displays; it should not have to write the shift itself.
#[test]
fn a_decoded_bitfield_channel_walks_its_named_bits() {
    let layout = bf_layout();
    let frame = layout.encode(&[]).value;

    let decoded = layout.decode(&frame);
    let bits: Vec<(&str, bool)> = decoded[0]
        .bits()
        .map(|(def, on)| (def.name.as_str(), on))
        .collect();

    // The channel default 0x0005 with bit 2 cleared and bit 7 set: 0x0081.
    assert_eq!(
        bits,
        vec![("alive", true), ("fault", false), ("ready", true)]
    );
}

// A channel with no bits named walks nothing, and says so quietly.
#[test]
fn a_channel_with_no_named_bits_walks_nothing() {
    let layout = bf_layout();
    let frame = layout.encode(&[]).value;

    assert_eq!(layout.decode(&frame)[1].bits().count(), 0);
}

// editing.md §1: the Table holds "the header row and every data row as
// verbatim cell strings". A grid editor needs to read both.
#[test]
fn a_table_hands_over_its_header_and_its_rows() {
    let table = ChTable::parse_with(
        "番号,バイト数,謎の列\n1,4,keep\n# note\n",
        &ColumnVocabulary::japanese(),
    )
    .unwrap();

    assert_eq!(
        table.header(),
        Some(["番号".to_string(), "バイト数".into(), "謎の列".into()].as_slice())
    );
    let rows: Vec<&[String]> = table.rows().collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][2], "keep");
    assert_eq!(rows[1][0], "# note");
    assert_eq!(table.row(1).map(|r| r[0].as_str()), Some("# note"));
    assert_eq!(table.row(9), None);
}

// format.md §2: a file read positionally has no header row to hand over.
#[test]
fn a_positional_table_has_no_header() {
    let table = ChTable::parse("1,4,,Sec,Name,UI,1,,\n").unwrap();

    assert_eq!(table.header(), None);
    assert_eq!(table.rows().count(), 1);
}

// The BF table is read the same way.
#[test]
fn a_bf_table_hands_over_its_grid_too() {
    let table = BfTable::parse("number,bit,name\n2,0,alive\n").unwrap();

    assert_eq!(table.header().map(|h| h.len()), Some(3));
    assert_eq!(table.rows().count(), 1);
}
