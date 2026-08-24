//! The surface `docs/spec/abi.md` states, checked against the sentences of
//! the specification it carries rather than against the implementation.
//!
//! ADR-0023: what crosses the ABI is every rule a consumer would otherwise
//! reimplement, so each test here names the rule it is standing in for.

use std::ffi::c_char;
use std::ptr;

use chdef_capi::*;

const CH: &str = "number,bytes,type,name,lsb,offset,unit,default,format\n\
                  1,2,BF,status,1,,,5,\n\
                  2,1,UI,speed,1,10,km/h,,HEX\n\
                  3,1,UI,temp,0.5,0,C,,DEC\n";

const BF: &str = "number,bit,name,default,memo\n\
                  1,0,ready,,keeps\n\
                  1,2,fault,0,cleared\n\
                  1,7,alarm,1,set\n";

struct Layout(*mut ChdefLayout);

impl Layout {
    fn parse() -> Layout {
        let mut layout = ptr::null_mut();
        let mut issues = ptr::null_mut();
        let status = unsafe {
            chdef_layout_parse(
                CH.as_ptr(),
                CH.len(),
                BF.as_ptr(),
                BF.len(),
                &mut layout,
                &mut issues,
                ptr::null_mut(),
                0,
            )
        };
        assert_eq!(status, CHDEF_OK);
        unsafe { chdef_issues_free(issues) };
        Layout(layout)
    }
}

impl Drop for Layout {
    fn drop(&mut self) {
        unsafe { chdef_layout_free(self.0) };
    }
}

/// Ask a text call for its length, then for its value — the two-call
/// pattern of abi.md §2.
fn text(mut call: impl FnMut(*mut c_char, usize) -> usize) -> String {
    let needed = call(ptr::null_mut(), 0);
    let mut buf = vec![0u8; needed + 1];
    let again = call(buf.as_mut_ptr() as *mut c_char, buf.len());
    assert_eq!(again, needed, "the second call reported a different length");
    buf.truncate(needed);
    String::from_utf8(buf).unwrap()
}

fn channel_of(layout: &Layout, index: usize) -> ChdefChannel {
    let mut ch = ChdefChannel::default();
    assert_eq!(
        unsafe { chdef_layout_channel_at(layout.0, index, &mut ch) },
        CHDEF_OK
    );
    ch
}

// ------------------------------------------------------------------- bits

#[test]
fn a_channels_bits_are_reachable_one_by_one_up_to_its_bit_count() {
    let layout = Layout::parse();
    assert_eq!(
        channel_of(&layout, 0).bit_count,
        3,
        "the BF CSV names three"
    );

    let mut bit = ChdefBit::default();
    assert_eq!(
        unsafe { chdef_layout_bit_at(layout.0, 0, 0, &mut bit) },
        CHDEF_OK
    );
    assert_eq!((bit.channel, bit.bit), (1, 0));
}

#[test]
fn a_bit_without_a_default_is_distinguishable_from_one_defaulting_to_zero() {
    // format.md §3: the BF `default` column is `0` / `1`, and empty means
    // the bit keeps the parent channel default. The two must not collapse.
    let layout = Layout::parse();
    let default_of = |index| {
        let mut bit = ChdefBit::default();
        assert_eq!(
            unsafe { chdef_layout_bit_at(layout.0, 0, index, &mut bit) },
            CHDEF_OK
        );
        bit.default_value
    };
    assert_eq!(default_of(0), -1, "bit 0 names no default");
    assert_eq!(default_of(1), 0, "bit 2 defaults to 0");
    assert_eq!(default_of(2), 1, "bit 7 defaults to 1");
}

#[test]
fn a_bit_name_and_memo_cross_as_text() {
    let layout = Layout::parse();
    let field = |index, which| {
        text(|buf, cap| unsafe { chdef_layout_bit_text(layout.0, 0, index, which, buf, cap) })
    };
    assert_eq!(field(0, CHDEF_BIT_NAME), "ready");
    assert_eq!(field(0, CHDEF_BIT_MEMO), "keeps");
    assert_eq!(field(2, CHDEF_BIT_NAME), "alarm");
}

#[test]
fn a_bit_index_past_the_bit_count_is_an_index_error() {
    // abi.md §2: out of range is CHDEF_ERR_INDEX, never a panic.
    let layout = Layout::parse();
    let mut bit = ChdefBit::default();
    assert_eq!(
        unsafe { chdef_layout_bit_at(layout.0, 0, 3, &mut bit) },
        CHDEF_ERR_INDEX
    );
    assert_eq!(
        unsafe { chdef_layout_bit_at(layout.0, 1, 0, &mut bit) },
        CHDEF_ERR_INDEX,
        "channel 2 names no bits"
    );
}

#[test]
fn the_bits_of_a_frame_come_back_in_one_pass() {
    // conversion.md §4: channel 1 defaults to 5 (bits 0 and 2 set); bit 0
    // names no default so it stays 1, bit 2 is cleared, bit 7 is set —
    // 0x81. conversion.md §6: a decoded frame names each bit value.
    let layout = Layout::parse();
    let mut frame = [0u8; 4];
    let mut len = 0usize;
    let mut issues = ptr::null_mut();
    assert_eq!(
        unsafe {
            chdef_encode(
                layout.0,
                ptr::null(),
                0,
                frame.as_mut_ptr(),
                frame.len(),
                &mut len,
                &mut issues,
            )
        },
        CHDEF_OK
    );
    unsafe { chdef_issues_free(issues) };
    assert_eq!(frame[0], 0x81, "the merged default of conversion.md §4");

    let total = unsafe { chdef_layout_bit_total(layout.0) } as usize;
    assert_eq!(total, 3);

    let mut out = vec![ChdefBitReading::default(); total];
    let mut count = 0usize;
    assert_eq!(
        unsafe {
            chdef_decode_bits(
                layout.0,
                frame.as_ptr(),
                len,
                out.as_mut_ptr(),
                out.len(),
                &mut count,
            )
        },
        CHDEF_OK
    );
    assert_eq!(count, 3);
    let seen: Vec<(u32, u32, i32)> = out.iter().map(|b| (b.channel, b.bit, b.value)).collect();
    assert_eq!(seen, vec![(1, 0, 1), (1, 2, 0), (1, 7, 1)]);
}

#[test]
fn decoding_bits_into_a_short_buffer_reports_the_count_and_writes_nothing() {
    // abi.md §2: a buffer too small is CHDEF_ERR_BUFFER with nothing written.
    let layout = Layout::parse();
    let frame = [0x81u8, 0, 0, 0];
    let mut out = [ChdefBitReading::default(); 1];
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
        CHDEF_ERR_BUFFER
    );
    assert_eq!(count, 3, "the caller learns the size to provide");
    assert_eq!(out[0], ChdefBitReading::default(), "nothing was written");
}

// -------------------------------------------------------------- a reading

#[test]
fn the_format_column_selects_which_reading_is_shown() {
    // conversion.md §7 / ADR-0015: DEC means the physical value, HEX the
    // raw one, and it affects no conversion.
    let layout = Layout::parse();

    let mut hex = ChdefValue::default();
    assert_eq!(
        unsafe { chdef_layout_channel_displayed(layout.0, 1, 20, &mut hex) },
        CHDEF_OK
    );
    assert_eq!((hex.channel, hex.kind, hex.raw), (2, 1, 20));

    let mut dec = ChdefValue::default();
    assert_eq!(
        unsafe { chdef_layout_channel_displayed(layout.0, 2, 20, &mut dec) },
        CHDEF_OK
    );
    assert_eq!((dec.channel, dec.kind), (3, 0));
    assert_eq!(dec.physical, 10.0, "raw 20 times lsb 0.5 plus offset 0");
}

#[test]
fn a_raw_reading_renders_in_hexadecimal_padded_to_the_channel_width() {
    // conversion.md §7: render is the physical value with the channel unit,
    // or the raw one in hexadecimal padded to the channel width.
    let layout = Layout::parse();
    let rendered =
        text(|buf, cap| unsafe { chdef_layout_channel_render(layout.0, 1, 20, buf, cap) });
    assert_eq!(rendered, "0x14", "one byte wide, so two hex digits");
}

#[test]
fn the_abi_renders_what_the_crate_renders() {
    // ADR-0023: the ABI is the rules of the crate, not a second
    // implementation of them.
    let layout = Layout::parse();
    let parsed = chdef::parse_ch_csv(CH).unwrap();
    let built = chdef::build_layout(parsed.value, Vec::new());

    for (index, channel) in built.value.channels.iter().enumerate() {
        for raw in [0u64, 1, 20, 255] {
            let through_the_abi = text(|buf, cap| unsafe {
                chdef_layout_channel_render(layout.0, index, raw, buf, cap)
            });
            assert_eq!(
                through_the_abi,
                channel.render(raw),
                "channel {} raw {raw}",
                channel.number
            );
        }
    }
}

// --------------------------------------------------------- value notation

#[test]
fn a_leading_0x_means_raw_and_anything_else_means_physical() {
    // format.md §3: `0x` / `0X` is a raw bit pattern, a plain number is a
    // physical value.
    let read = |s: &str| {
        let mut value = ChdefValue::default();
        let status = unsafe { chdef_value_parse(s.as_ptr(), s.len(), 7, &mut value) };
        (status, value)
    };

    let (status, raw) = read("0x14");
    assert_eq!(status, CHDEF_OK);
    assert_eq!((raw.channel, raw.kind, raw.raw), (7, 1, 20));

    let (status, physical) = read("-1.5");
    assert_eq!(status, CHDEF_OK);
    assert_eq!((physical.channel, physical.kind), (7, 0));
    assert_eq!(physical.physical, -1.5);

    let (status, _) = read("0X14");
    assert_eq!(status, CHDEF_OK, "the prefix is case-insensitive");
}

#[test]
fn text_that_denotes_no_value_is_reported_rather_than_guessed() {
    let mut value = ChdefValue::default();
    let source = "not a number";
    assert_eq!(
        unsafe { chdef_value_parse(source.as_ptr(), source.len(), 1, &mut value) },
        CHDEF_ERR_VALUE
    );
}

// ------------------------------------------------------------------- grid

const GRID_CSV: &str = "\u{FEFF}number,bytes,memo\r\n1,2,first\r\n# a comment\r\n";

struct Grid(*mut ChdefGrid);

impl Grid {
    fn parse(source: &str) -> Grid {
        let mut grid = ptr::null_mut();
        let status = unsafe {
            chdef_grid_parse(source.as_ptr(), source.len(), &mut grid, ptr::null_mut(), 0)
        };
        assert_eq!(status, CHDEF_OK);
        Grid(grid)
    }

    fn to_csv(&self) -> String {
        text(|buf, cap| unsafe { chdef_grid_to_csv(self.0, buf, cap) })
    }

    fn cell(&self, row: usize, col: usize) -> String {
        text(|buf, cap| unsafe { chdef_grid_cell(self.0, row, col, buf, cap) })
    }
}

impl Drop for Grid {
    fn drop(&mut self) {
        unsafe { chdef_grid_free(self.0) };
    }
}

#[test]
fn a_grid_is_the_header_and_every_row_including_comments() {
    // editing.md §3: the header row and every data row, comment and blank
    // rows included; cells are 0-based with the header excluded.
    let grid = Grid::parse(GRID_CSV);
    assert_eq!(unsafe { chdef_grid_header_count(grid.0) }, 3);
    assert_eq!(unsafe { chdef_grid_row_count(grid.0) }, 2);
    assert_eq!(
        text(|buf, cap| unsafe { chdef_grid_header_at(grid.0, 2, buf, cap) }),
        "memo"
    );
    assert_eq!(grid.cell(0, 2), "first");
    assert_eq!(grid.cell(1, 0), "# a comment", "a comment row is a row");
}

#[test]
fn a_file_already_in_the_write_rules_round_trips_byte_for_byte() {
    // editing.md §2, including the byte-order mark and the record separator.
    let grid = Grid::parse(GRID_CSV);
    assert_eq!(grid.to_csv(), GRID_CSV);
}

#[test]
fn setting_a_cell_past_the_end_of_a_short_row_pads_it() {
    // editing.md §3.
    let grid = Grid::parse(GRID_CSV);
    let value = "late";
    assert_eq!(
        unsafe { chdef_grid_set_cell(grid.0, 1, 3, value.as_ptr(), value.len()) },
        CHDEF_OK
    );
    assert_eq!(unsafe { chdef_grid_col_count(grid.0, 1) }, 4);
    assert_eq!(grid.cell(1, 1), "");
    assert_eq!(grid.cell(1, 3), "late");
}

#[test]
fn setting_a_cell_outside_the_grid_is_an_index_error() {
    // abi.md §2: out of range is CHDEF_ERR_INDEX. A row is grown with
    // insert or append, never by writing past the last one.
    let grid = Grid::parse(GRID_CSV);
    let value = "stray";
    assert_eq!(
        unsafe { chdef_grid_set_cell(grid.0, 9, 0, value.as_ptr(), value.len()) },
        CHDEF_ERR_INDEX
    );
    assert_eq!(
        unsafe { chdef_grid_row_count(grid.0) },
        2,
        "nothing was added"
    );
}

#[test]
fn a_file_kept_without_a_bom_and_with_lf_endings_keeps_them() {
    // editing.md §2: the byte-order mark and the record separator come back
    // as they were read, so editing one cell of such a file does not
    // rewrite every line of it.
    let source = "number,bytes
1,2
";
    let grid = Grid::parse(source);
    assert_eq!(grid.to_csv(), source);

    let value = "4";
    assert_eq!(
        unsafe { chdef_grid_set_cell(grid.0, 0, 1, value.as_ptr(), value.len()) },
        CHDEF_OK
    );
    assert_eq!(
        grid.to_csv(),
        "number,bytes
1,4
"
    );
}

#[test]
fn a_row_is_inserted_empty_and_filled_with_cell_writes() {
    // ADR-0023: passing an array of strings buys nothing over the set_cell
    // that has to exist anyway.
    let grid = Grid::parse(GRID_CSV);
    assert_eq!(unsafe { chdef_grid_insert_row(grid.0, 0) }, CHDEF_OK);
    assert_eq!(unsafe { chdef_grid_row_count(grid.0) }, 3);
    assert_eq!(unsafe { chdef_grid_col_count(grid.0, 0) }, 0);

    let value = "2";
    assert_eq!(
        unsafe { chdef_grid_set_cell(grid.0, 0, 0, value.as_ptr(), value.len()) },
        CHDEF_OK
    );
    assert_eq!(grid.cell(0, 0), "2");
    assert_eq!(grid.cell(1, 2), "first", "the old first row moved down");
}

#[test]
fn removing_a_row_outside_the_grid_removes_nothing_and_says_so() {
    // editing.md §3: the row operations are total — never a panic.
    let grid = Grid::parse(GRID_CSV);
    assert_eq!(unsafe { chdef_grid_remove_row(grid.0, 9) }, CHDEF_ERR_INDEX);
    assert_eq!(unsafe { chdef_grid_row_count(grid.0) }, 2);
    assert_eq!(unsafe { chdef_grid_remove_row(grid.0, 0) }, CHDEF_OK);
    assert_eq!(unsafe { chdef_grid_row_count(grid.0) }, 1);
    assert_eq!(grid.cell(0, 0), "# a comment");
}

#[test]
fn appending_a_row_puts_it_after_the_last() {
    let grid = Grid::parse(GRID_CSV);
    assert_eq!(unsafe { chdef_grid_append_row(grid.0) }, CHDEF_OK);
    assert_eq!(unsafe { chdef_grid_row_count(grid.0) }, 3);
    assert_eq!(unsafe { chdef_grid_col_count(grid.0, 2) }, 0);
}

#[test]
fn a_cell_outside_the_grid_is_empty_rather_than_a_crash() {
    let grid = Grid::parse(GRID_CSV);
    assert_eq!(grid.cell(9, 9), "");
    assert_eq!(unsafe { chdef_grid_col_count(grid.0, 9) }, 0);
}

#[test]
fn a_freed_grid_handle_is_reported_rather_than_dereferenced() {
    // abi.md §2: a stale handle is CHDEF_ERR_HANDLE.
    let mut grid = ptr::null_mut();
    assert_eq!(
        unsafe {
            chdef_grid_parse(
                GRID_CSV.as_ptr(),
                GRID_CSV.len(),
                &mut grid,
                ptr::null_mut(),
                0,
            )
        },
        CHDEF_OK
    );
    unsafe { chdef_grid_free(grid) };
    assert_eq!(unsafe { chdef_grid_row_count(grid) }, 0);
    assert_eq!(unsafe { chdef_grid_remove_row(grid, 0) }, CHDEF_ERR_HANDLE);
    unsafe { chdef_grid_free(grid) };
}

// ---------------------------------------------------------------- version

#[test]
fn the_version_rose_when_the_surface_grew() {
    // abi.md §4: CHDEF_ABI_VERSION increments on every added or changed
    // symbol, and a caller checks it is at least what it needs.
    assert!(chdef_abi_version() >= 2);
    assert_eq!(chdef_abi_version(), CHDEF_ABI_VERSION);
}
