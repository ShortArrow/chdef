//! Derived channels (`docs/spec/format.md` §6): a channel chdef computes
//! from the rest of the frame, by a recipe the file states.
//!
//! ADR-0029. The expected CRCs here were computed by a separate
//! implementation of the Rocksoft model, so a mistake shared with the
//! crate's own code does not pass unnoticed.

use chdef::*;

/// Sync, counter, payload, then a CRC over channels 1..3. Little-endian,
/// so the body is `7e 7e 01 00 2a` and the CRC is 0x9BBF.
const CH: &str = "number,bytes,type,kind,derived,default,name\n\
                  1,2,UI,const,,0x7E7E,SYNC\n\
                  2,2,UI,counter,,1,FRAME_NO\n\
                  3,1,UI,plain,,42,PAYLOAD\n\
                  4,2,UI,derived,crc16/x25 1..3,,CRC\n";

const BODY: [u8; 5] = [0x7E, 0x7E, 0x01, 0x00, 0x2A];
const CRC_OVER_1_3: u16 = 0x9BBF;

fn layout_of(source: &str) -> Parsed<ChannelLayout> {
    let parsed = parse_ch_csv(source).unwrap();
    let mut issues = parsed.issues;
    let built = build_layout(parsed.value, Vec::new());
    issues.extend(built.issues);
    Parsed {
        value: built.value,
        issues,
    }
}

fn layout() -> ChannelLayout {
    let read = layout_of(CH);
    assert!(read.issues.is_empty(), "{:?}", read.issues);
    read.value
}

// ------------------------------------------------------------ the recipe

#[test]
fn a_named_recipe_is_the_six_numbers_it_stands_for() {
    // §6: `crc16/x25` expands to exactly these, and a name has no standing
    // the numbers lack.
    let named = layout();
    let spelled = layout_of(
        "number,bytes,type,kind,derived,default,name\n\
         1,2,UI,const,,0x7E7E,SYNC\n\
         2,2,UI,counter,,1,FRAME_NO\n\
         3,1,UI,plain,,42,PAYLOAD\n\
         4,2,UI,derived,crc16 poly=0x1021 init=0xFFFF refin=1 refout=1 xorout=0xFFFF 1..3,,CRC\n",
    );
    assert!(spelled.issues.is_empty(), "{:?}", spelled.issues);

    let mut a = named.encode(&[]).value;
    let mut b = spelled.value.encode(&[]).value;
    assert!(named.seal(&mut a).is_empty());
    assert!(spelled.value.seal(&mut b).is_empty());
    assert_eq!(a, b, "the name and the numbers agree");
}

#[test]
fn a_recipe_without_a_range_is_reported_rather_than_assumed() {
    // §6: there is no default coverage. Guessing would be right often and
    // silently wrong otherwise.
    let read = layout_of(
        "number,bytes,type,kind,derived,name\n\
         1,2,UI,plain,,A\n\
         2,2,UI,derived,crc16/x25,CRC\n",
    );
    assert!(
        read.issues
            .iter()
            .any(|i| i.code == IssueCode::DerivedInvalid),
        "{:?}",
        read.issues
    );
}

#[test]
fn an_unreadable_cell_and_an_unimplemented_algorithm_are_different_findings() {
    // The distinction the escape hatch rests on. A cell whose syntax fails
    // has no coverage to hand anyone; a cell naming an algorithm chdef
    // does not compute has one, so it must not be thrown away with it.
    let broken = layout_of(
        "number,bytes,type,kind,derived,name
         1,2,UI,plain,,A
         2,2,UI,derived,crc16/x25 nonsense,CRC
",
    );
    let issue = broken
        .issues
        .iter()
        .find(|i| i.code == IssueCode::DerivedInvalid)
        .unwrap_or_else(|| panic!("{:?}", broken.issues));
    assert_eq!(issue.channel, Some(2));
    assert_eq!(issue.found.as_deref(), Some("crc16/x25 nonsense"));
    assert_eq!(
        broken.value.covered_bytes(2, &[0; 4]),
        None,
        "there is nothing to hand over"
    );

    let unimplemented = layout_of(
        "number,bytes,type,kind,derived,name
         1,2,UI,plain,,A
         2,2,UI,derived,md5 1..1,CRC
",
    );
    assert!(
        unimplemented.issues.is_empty(),
        "the cell reads: {:?}",
        unimplemented.issues
    );
    assert_eq!(
        unimplemented
            .value
            .covered_bytes(2, &[0; 4])
            .map(|b| b.len()),
        Some(2),
        "and its coverage is usable"
    );
}

#[test]
fn the_column_is_read_only_for_a_derived_channel() {
    // §3: read only for `kind` = `derived`, and ignored otherwise.
    let read = layout_of(
        "number,bytes,type,kind,derived,name\n\
         1,2,UI,plain,nonsense,A\n",
    );
    assert!(read.issues.is_empty(), "{:?}", read.issues);
}

// ----------------------------------------------------------- the sealing

#[test]
fn encode_leaves_a_derived_channel_at_its_default() {
    // ADR-0029: encode is untouched and stays a pure function of the
    // layout and the values given.
    let frame = layout().encode(&[]).value;

    assert_eq!(&frame[..5], &BODY, "the other channels are written");
    assert_eq!(&frame[5..], &[0, 0], "the CRC is not computed here");
}

#[test]
fn sealing_fills_the_derived_channel() {
    let layout = layout();
    let mut frame = layout.encode(&[]).value;

    let issues = layout.seal(&mut frame);

    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(&frame[5..], &CRC_OVER_1_3.to_le_bytes());
    assert_eq!(&frame[..5], &BODY, "nothing else moved");
}

#[test]
fn sealing_twice_writes_the_same_frame() {
    // The recipe reads the bytes as they will be sent, and the derived
    // channel is not among the ones it covers here.
    let layout = layout();
    let mut frame = layout.encode(&[]).value;

    layout.seal(&mut frame);
    let once = frame.clone();
    layout.seal(&mut frame);

    assert_eq!(frame, once);
}

#[test]
fn sealing_follows_the_values_that_were_encoded() {
    let layout = layout();
    let mut a = layout.encode(&[(2, Value::Raw(1))]).value;
    let mut b = layout.encode(&[(2, Value::Raw(2))]).value;
    layout.seal(&mut a);
    layout.seal(&mut b);

    assert_ne!(a[5..], b[5..], "a different body seals differently");
    assert_eq!(&a[5..], &CRC_OVER_1_3.to_le_bytes());
}

#[test]
fn a_frame_too_short_to_hold_the_channel_is_reported() {
    let layout = layout();
    let mut frame = vec![0u8; 4];

    let issues = layout.seal(&mut frame);

    assert!(!issues.is_empty(), "a short frame cannot be sealed");
    assert_eq!(frame, vec![0u8; 4], "and nothing was written");
}

#[test]
fn a_recipe_covering_a_channel_the_layout_lacks_computes_nothing() {
    let read = layout_of(
        "number,bytes,type,kind,derived,name\n\
         1,2,UI,plain,,A\n\
         2,2,UI,derived,crc16/x25 1..9,CRC\n",
    );
    let mut frame = read.value.encode(&[]).value;
    let issues = read.value.seal(&mut frame);

    assert!(
        issues
            .iter()
            .any(|i| i.code == IssueCode::DerivedUnknownChannel),
        "{issues:?}"
    );
    assert_eq!(&frame[2..], &[0, 0], "nothing was computed");
}

#[test]
fn spans_may_be_listed_when_the_coverage_is_not_one_run() {
    // §6: `2..3,5..7` — each span inclusive, covered as written.
    let split = layout_of(
        "number,bytes,type,kind,derived,default,name\n\
         1,2,UI,const,,0x7E7E,SYNC\n\
         2,2,UI,counter,,1,FRAME_NO\n\
         3,1,UI,plain,,42,PAYLOAD\n\
         4,2,UI,derived,\"crc16/x25 1..2,3..3\",,CRC\n",
    );
    assert!(split.issues.is_empty(), "{:?}", split.issues);

    let mut frame = split.value.encode(&[]).value;
    assert!(split.value.seal(&mut frame).is_empty());
    assert_eq!(
        &frame[5..],
        &CRC_OVER_1_3.to_le_bytes(),
        "1..2 then 3..3 covers the same bytes as 1..3"
    );
}

// ----------------------------------------------------------- the checking

#[test]
fn a_sealed_frame_checks_out() {
    let layout = layout();
    let mut frame = layout.encode(&[]).value;
    layout.seal(&mut frame);

    assert!(layout.derived_mismatches(&frame).is_empty());
}

#[test]
fn an_unsealed_frame_is_named_as_wrong() {
    let layout = layout();
    let frame = layout.encode(&[]).value;

    let issues = layout.derived_mismatches(&frame);

    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, IssueCode::DerivedMismatch);
    assert_eq!(issues[0].channel, Some(4));
    assert_eq!(issues[0].found.as_deref(), Some("0x0000"), "what it holds");
    assert_eq!(
        issues[0].used.as_deref(),
        Some(&format!("0x{CRC_OVER_1_3:04X}")[..]),
        "what the recipe computes"
    );
}

#[test]
fn a_frame_corrupted_anywhere_it_covers_is_named() {
    let layout = layout();
    let mut frame = layout.encode(&[]).value;
    layout.seal(&mut frame);
    frame[4] ^= 0xFF;

    assert_eq!(layout.derived_mismatches(&frame).len(), 1);
}

#[test]
fn checking_changes_nothing() {
    let layout = layout();
    let mut frame = layout.encode(&[]).value;
    layout.seal(&mut frame);
    let before = frame.clone();

    let _ = layout.derived_mismatches(&frame);

    assert_eq!(frame, before);
}

#[test]
fn a_layout_with_no_derived_channel_has_nothing_to_seal_or_check() {
    let read = layout_of("number,bytes,type,default\n1,2,UI,7\n");
    let mut frame = read.value.encode(&[]).value;
    let before = frame.clone();

    assert!(read.value.seal(&mut frame).is_empty());
    assert_eq!(frame, before);
    assert!(read.value.derived_mismatches(&frame).is_empty());
}

// ------------------------------------------- the storey below sealing

#[test]
fn a_recipe_chdef_cannot_compute_still_says_what_it_covers() {
    // ADR-0029 / library-design: a device whose checksum chdef never heard
    // of is not blocked. The algorithm is the caller's; which bytes it
    // covers is chdef's, and that is what is handed over.
    let read = layout_of(
        "number,bytes,type,kind,derived,default,name
         1,2,UI,const,,0x7E7E,SYNC
         2,2,UI,counter,,1,FRAME_NO
         3,1,UI,plain,,42,PAYLOAD
         4,2,UI,derived,fletcher16 1..3,,SUM
",
    );
    assert!(read.issues.is_empty(), "the cell reads: {:?}", read.issues);
    let layout = read.value;

    let mut frame = layout.encode(&[]).value;
    assert_eq!(
        layout.covered_bytes(4, &frame).as_deref(),
        Some(&BODY[..]),
        "the coverage is known even though the algorithm is not"
    );

    // Sealing says so rather than guessing or staying silent.
    let issues = layout.seal(&mut frame);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].code, IssueCode::DerivedUnknownRecipe);
    assert_eq!(issues[0].found.as_deref(), Some("fletcher16"));
    assert_eq!(&frame[5..], &[0, 0], "and computes nothing");

    // The caller fills it with its own, through the ordinary door.
    let mine: u16 = layout
        .covered_bytes(4, &frame)
        .unwrap()
        .iter()
        .fold(0u16, |sum, b| sum.wrapping_add(*b as u16));
    let sealed = layout.encode(&[(4, Value::Raw(mine as u64))]).value;
    assert_eq!(&sealed[5..], &mine.to_le_bytes());
}

#[test]
fn covered_bytes_is_the_span_the_recipe_names_and_nothing_else() {
    let layout = layout();
    let frame = layout.encode(&[]).value;

    assert_eq!(layout.covered_bytes(4, &frame).as_deref(), Some(&BODY[..]));
    assert_eq!(
        layout.covered_bytes(1, &frame),
        None,
        "not a derived channel"
    );
    assert_eq!(
        layout.covered_bytes(9, &frame),
        None,
        "not a channel at all"
    );
    assert_eq!(layout.covered_bytes(4, &frame[..2]), None, "too short");
}

// ------------------------------------------------------ the vocabulary

#[test]
fn the_recipes_this_chdef_knows_are_enumerable() {
    // ADR-0026: where chdef says the set can grow, it says what is in it.
    let names = DerivedRecipe::all();
    assert!(!names.is_empty());
    assert!(names.contains(&"crc16/x25"), "{names:?}");
}
