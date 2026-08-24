//! A C ABI over [`chdef`]: read CH / BF definitions, describe the frame
//! layout, encode and decode frames (ADR-0021).
//!
//! It is a codec, not the crate. Reading definitions, describing the
//! layout and converting frames cross the boundary; the Table stage —
//! editing a definition and writing it back — deliberately does not.
//!
//! Three rules shape every signature:
//!
//! - **No enums.** Every identity crosses as its stable ASCII string (an
//!   Issue's code, a channel's `type`), because both vocabularies are
//!   documented as growing. `endian` is the one exception: two values that
//!   cannot grow.
//! - **Strings go into the caller's buffer.** A `*_text` call writes UTF-8
//!   into `buf` and returns the length the value needs, so nothing chdef
//!   produces is ever the caller's to free.
//! - **Absent numbers are `-1`.** `row`, `col`, `channel`, `bit` and a
//!   channel's `default` are non-negative when they exist.
//!
//! Every entry point catches panics and reports [`CHDEF_PANIC`]; a panic
//! across `extern "C"` would otherwise be undefined behaviour, and the
//! host process is the caller's, not chdef's to abort.

use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use chdef::{
    build_layout, BitFieldDef, ChannelLayout, ChdefError, Endian, Grid, Issue, IssueCode, Value,
};

/// The revision of this ABI. It rises whenever a symbol is added or
/// changed; a caller checks it is **at least** the value its declarations
/// were written for before calling anything else. Symbols are added and
/// never withdrawn, so a newer library serves an older caller.
pub const CHDEF_ABI_VERSION: u32 = 2;

// ---------------------------------------------------------------- statuses

/// The call succeeded.
pub const CHDEF_OK: i32 = 0;
/// A handle was null, or did not come from this library.
pub const CHDEF_ERR_HANDLE: i32 = -1;
/// An index was past the end of what it indexes.
pub const CHDEF_ERR_INDEX: i32 = -2;
/// A caller-provided buffer was too small; the out-length says what the
/// call needs.
pub const CHDEF_ERR_BUFFER: i32 = -3;
/// A pointer that must not be null was.
pub const CHDEF_ERR_NULL: i32 = -4;
/// The bytes handed in are not UTF-8.
pub const CHDEF_ERR_UTF8: i32 = -5;
/// The CSV is structurally broken (an unterminated quote).
pub const CHDEF_ERR_CSV: i32 = -6;
/// The file could not be read.
pub const CHDEF_ERR_IO: i32 = -7;

/// The text denotes no value in the notation of
/// `docs/spec/format.md` §3.
pub const CHDEF_ERR_VALUE: i32 = -8;
/// A panic was caught at the boundary. A bug in chdef; the handles the
/// call touched are left as they were.
pub const CHDEF_PANIC: i32 = -99;

/// Byte order values for [`chdef_layout_set_endian`].
pub const CHDEF_LITTLE: i32 = 0;
/// See [`CHDEF_LITTLE`].
pub const CHDEF_BIG: i32 = 1;

// ------------------------------------------------------------- text fields

/// A channel's `name`.
pub const CHDEF_CHANNEL_NAME: i32 = 0;
/// A channel's `type`, as `UI` / `SI` / `BF`.
pub const CHDEF_CHANNEL_TYPE: i32 = 1;
/// A channel's `unit`.
pub const CHDEF_CHANNEL_UNIT: i32 = 2;
/// A channel's `section`.
pub const CHDEF_CHANNEL_SECTION: i32 = 3;
/// A channel's `memo`.
pub const CHDEF_CHANNEL_MEMO: i32 = 4;
/// A channel's `var`.
pub const CHDEF_CHANNEL_VAR: i32 = 5;
/// Which reading the channel shows, as `DEC` / `HEX`.
pub const CHDEF_CHANNEL_FORMAT: i32 = 6;
/// A channel's `min`, in the notation its cell used; empty when unstated.
pub const CHDEF_CHANNEL_MIN: i32 = 7;
/// A channel's `max`. See [`CHDEF_CHANNEL_MIN`].
pub const CHDEF_CHANNEL_MAX: i32 = 8;

/// An Issue's stable ASCII code.
pub const CHDEF_ISSUE_CODE: i32 = 0;
/// The value chdef could not use, as the file spelled it.
pub const CHDEF_ISSUE_FOUND: i32 = 1;
/// The value chdef used instead.
pub const CHDEF_ISSUE_USED: i32 = 2;
/// The English sentence. Its wording is not part of the contract.
pub const CHDEF_ISSUE_MESSAGE: i32 = 3;

/// The bit name, for [`chdef_layout_bit_text`].
pub const CHDEF_BIT_NAME: i32 = 0;

/// The bit memo, for [`chdef_layout_bit_text`].
pub const CHDEF_BIT_MEMO: i32 = 1;

// ----------------------------------------------------------- plain records

/// One channel of the layout. `default_value` is `-1` when the channel
/// states none.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ChdefChannel {
    pub number: u32,
    pub at: u64,
    pub bytes: u64,
    pub lsb: f64,
    pub offset: f64,
    pub default_value: i64,
    pub favorite: i32,
    pub bit_count: u64,
}

/// One diagnostic. `row`, `col`, `channel` and `bit` are `-1` when the
/// finding carries none.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChdefIssue {
    pub row: i64,
    pub col: i64,
    pub channel: i64,
    pub bit: i64,
}

/// A value handed to [`chdef_encode`]. `kind` picks which of the two
/// number fields is meant.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ChdefValue {
    pub channel: u32,
    /// `0` — take `physical`. `1` — take `raw`.
    pub kind: i32,
    pub physical: f64,
    pub raw: u64,
}

impl ChdefValue {
    /// A physical value for one channel.
    pub fn physical(channel: u32, physical: f64) -> ChdefValue {
        ChdefValue {
            channel,
            kind: 0,
            physical,
            raw: 0,
        }
    }

    /// A raw bit pattern for one channel.
    pub fn raw(channel: u32, raw: u64) -> ChdefValue {
        ChdefValue {
            channel,
            kind: 1,
            physical: 0.0,
            raw,
        }
    }

    fn into_value(self) -> Value {
        if self.kind == 1 {
            Value::Raw(self.raw)
        } else {
            Value::Physical(self.physical)
        }
    }
}

/// One named bit of a channel. `default_value` is `0` or `1`, or `-1`
/// when the BF row names none and the bit keeps the parent channel one.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChdefBit {
    pub channel: u32,
    pub bit: u32,
    pub default_value: i32,
}

/// One named bit of a decoded frame, and whether it is set.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChdefBitReading {
    pub channel: u32,
    pub bit: u32,
    pub value: i32,
}

/// One channel's readings from a decoded frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ChdefReading {
    pub channel: u32,
    pub raw: u64,
    pub value: f64,
}

// ---------------------------------------------------------------- handles

const LAYOUT_TAG: u64 = 0x6368_6465_665f_6c79; // "chdef_ly"
const ISSUES_TAG: u64 = 0x6368_6465_665f_6973; // "chdef_is"
const GRID_TAG: u64 = 0x6368_6465_665f_6772; // "chdef_gr"

/// An opaque frame layout. Freed once with [`chdef_layout_free`].
pub struct ChdefLayout {
    tag: u64,
    layout: ChannelLayout,
}

/// An opaque grid of cells. Freed once with [`chdef_grid_free`].
pub struct ChdefGrid {
    tag: u64,
    grid: Grid,
}

/// An opaque list of diagnostics. Freed once with [`chdef_issues_free`].
pub struct ChdefIssues {
    tag: u64,
    issues: Vec<Issue>,
}

/// Read a handle, checking that it is one of ours.
unsafe fn layout_of(handle: *const ChdefLayout) -> Option<&'static ChannelLayout> {
    let handle = handle.as_ref()?;
    (handle.tag == LAYOUT_TAG).then_some(&handle.layout)
}

unsafe fn layout_mut(handle: *mut ChdefLayout) -> Option<&'static mut ChannelLayout> {
    let handle = handle.as_mut()?;
    (handle.tag == LAYOUT_TAG).then_some(&mut handle.layout)
}

unsafe fn grid_of(handle: *const ChdefGrid) -> Option<&'static Grid> {
    let handle = handle.as_ref()?;
    (handle.tag == GRID_TAG).then_some(&handle.grid)
}

unsafe fn grid_mut(handle: *mut ChdefGrid) -> Option<&'static mut Grid> {
    let handle = handle.as_mut()?;
    (handle.tag == GRID_TAG).then_some(&mut handle.grid)
}

unsafe fn issues_of(handle: *const ChdefIssues) -> Option<&'static [Issue]> {
    let handle = handle.as_ref()?;
    (handle.tag == ISSUES_TAG).then_some(handle.issues.as_slice())
}

fn into_issues(issues: Vec<Issue>) -> *mut ChdefIssues {
    Box::into_raw(Box::new(ChdefIssues {
        tag: ISSUES_TAG,
        issues,
    }))
}

/// Run `body`, reporting a panic as [`CHDEF_PANIC`] rather than letting it
/// cross `extern "C"`.
fn guard(body: impl FnOnce() -> i32) -> i32 {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(CHDEF_PANIC)
}

/// Run `body`, reporting a panic as `0`.
fn guard_len(body: impl FnOnce() -> usize) -> usize {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(0)
}

/// Write `text` as UTF-8 into `buf`, always terminating, and return the
/// length `text` needs. A `buf` of zero capacity writes nothing, so a
/// caller can ask the length first.
unsafe fn write_text(text: &str, buf: *mut c_char, cap: usize) -> usize {
    if !buf.is_null() && cap > 0 {
        let room = cap - 1;
        let mut end = text.len().min(room);
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        std::ptr::copy_nonoverlapping(text.as_ptr(), buf as *mut u8, end);
        *buf.add(end) = 0;
    }
    text.len()
}

/// The bytes of a caller's string, or `None` when the pointer is null.
unsafe fn borrow_str(ptr: *const u8, len: usize) -> Option<Result<&'static str, ()>> {
    if ptr.is_null() {
        return None;
    }
    Some(std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).map_err(|_| ()))
}

fn status_of(error: &ChdefError) -> i32 {
    match error {
        ChdefError::Io { .. } => CHDEF_ERR_IO,
        ChdefError::CsvParse { .. } => CHDEF_ERR_CSV,
        ChdefError::Encoding { .. } => CHDEF_ERR_UTF8,
        _ => CHDEF_ERR_CSV,
    }
}

// ------------------------------------------------------------------- entry

/// The revision of this ABI ([`CHDEF_ABI_VERSION`]).
#[no_mangle]
pub extern "C" fn chdef_abi_version() -> u32 {
    CHDEF_ABI_VERSION
}

/// Read a CH CSV and an optional BF CSV into a layout.
///
/// `ch` / `bf` are UTF-8 text of the given lengths; `bf` may be null with
/// a length of 0. On success `out_layout` and `out_issues` receive handles
/// the caller frees, and the status is [`CHDEF_OK`]. On failure both are
/// left null and, when `err_buf` is not null, the error's message is
/// written into it.
///
/// # Safety
///
/// The pointers must be valid for the lengths given, and the out-params
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_parse(
    ch: *const u8,
    ch_len: usize,
    bf: *const u8,
    bf_len: usize,
    out_layout: *mut *mut ChdefLayout,
    out_issues: *mut *mut ChdefIssues,
    err_buf: *mut c_char,
    err_cap: usize,
) -> i32 {
    guard(|| {
        if out_layout.is_null() || out_issues.is_null() {
            return CHDEF_ERR_NULL;
        }
        *out_layout = std::ptr::null_mut();
        *out_issues = std::ptr::null_mut();

        let ch_text = match borrow_str(ch, ch_len) {
            None => return CHDEF_ERR_NULL,
            Some(Err(())) => {
                write_text("the CH definition is not UTF-8", err_buf, err_cap);
                return CHDEF_ERR_UTF8;
            }
            Some(Ok(text)) => text,
        };
        let bf_text = match borrow_str(bf, bf_len) {
            None => "",
            Some(Err(())) => {
                write_text("the BF definition is not UTF-8", err_buf, err_cap);
                return CHDEF_ERR_UTF8;
            }
            Some(Ok(text)) => text,
        };

        let channels = match chdef::parse_ch_csv(ch_text) {
            Ok(parsed) => parsed,
            Err(e) => {
                write_text(&e.to_string(), err_buf, err_cap);
                return status_of(&e);
            }
        };
        let bitfields = match chdef::parse_bf_csv(bf_text) {
            Ok(parsed) => parsed,
            Err(e) => {
                write_text(&e.to_string(), err_buf, err_cap);
                return status_of(&e);
            }
        };

        let built = build_layout(channels.value, bitfields.value);
        let mut issues = channels.issues;
        issues.extend(bitfields.issues);
        issues.extend(built.issues);

        *out_layout = Box::into_raw(Box::new(ChdefLayout {
            tag: LAYOUT_TAG,
            layout: built.value,
        }));
        *out_issues = into_issues(issues);
        CHDEF_OK
    })
}

/// Release a layout. A null handle is ignored.
///
/// # Safety
///
/// The handle must come from [`chdef_layout_parse`] and must not have been
/// freed already.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_free(handle: *mut ChdefLayout) {
    guard(|| {
        if let Some(h) = handle.as_mut() {
            if h.tag == LAYOUT_TAG {
                h.tag = 0;
                drop(Box::from_raw(handle));
            }
        }
        CHDEF_OK
    });
}

/// Release a list of diagnostics. A null handle is ignored.
///
/// # Safety
///
/// The handle must come from this library and must not have been freed
/// already.
#[no_mangle]
pub unsafe extern "C" fn chdef_issues_free(handle: *mut ChdefIssues) {
    guard(|| {
        if let Some(h) = handle.as_mut() {
            if h.tag == ISSUES_TAG {
                h.tag = 0;
                drop(Box::from_raw(handle));
            }
        }
        CHDEF_OK
    });
}

/// The data length of the frame in bytes, or `0` for an unusable handle.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_total_bytes(handle: *const ChdefLayout) -> u64 {
    guard_len(|| layout_of(handle).map(|l| l.total_bytes()).unwrap_or(0)) as u64
}

/// How many channels the layout has, or `0` for an unusable handle.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_channel_count(handle: *const ChdefLayout) -> u64 {
    guard_len(|| layout_of(handle).map(|l| l.channels.len()).unwrap_or(0)) as u64
}

/// Describe the channel at `index` in numbers.
///
/// # Safety
///
/// The handle must be null or one of ours, and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_channel_at(
    handle: *const ChdefLayout,
    index: usize,
    out: *mut ChdefChannel,
) -> i32 {
    guard(|| {
        let Some(layout) = layout_of(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        if out.is_null() {
            return CHDEF_ERR_NULL;
        }
        let Some((at, ch)) = layout.positions().nth(index) else {
            return CHDEF_ERR_INDEX;
        };
        *out = ChdefChannel {
            number: ch.number,
            at: at as u64,
            bytes: ch.width() as u64,
            lsb: ch.lsb,
            offset: ch.offset,
            default_value: layout
                .channel_default(ch.number)
                .filter(|_| ch.default_value.is_some() || ch.data_type.is_bitfield())
                .map(|d| d as i64)
                .unwrap_or(-1),
            favorite: i32::from(ch.favorite),
            bit_count: layout
                .bitfields
                .iter()
                .filter(|b| b.parent_channel == ch.number)
                .count() as u64,
        };
        CHDEF_OK
    })
}

/// Write one of a channel's text fields into `buf`; see
/// [`CHDEF_CHANNEL_NAME`] and the constants beside it. Returns the length
/// the value needs, or `0` for an unusable handle, index or field.
///
/// # Safety
///
/// The handle must be null or one of ours, and `buf` writable for `cap`.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_channel_text(
    handle: *const ChdefLayout,
    index: usize,
    field: i32,
    buf: *mut c_char,
    cap: usize,
) -> usize {
    guard_len(|| {
        let Some(layout) = layout_of(handle) else {
            return 0;
        };
        let Some(ch) = layout.channels.get(index) else {
            return 0;
        };
        let owned;
        let text: &str = match field {
            CHDEF_CHANNEL_NAME => &ch.name,
            CHDEF_CHANNEL_TYPE => ch.data_type.as_str(),
            CHDEF_CHANNEL_UNIT => &ch.unit,
            CHDEF_CHANNEL_SECTION => &ch.section,
            CHDEF_CHANNEL_MEMO => &ch.memo,
            CHDEF_CHANNEL_VAR => &ch.var,
            CHDEF_CHANNEL_FORMAT => ch.format.as_str(),
            CHDEF_CHANNEL_MIN => {
                owned = ch.min.map(|v| v.to_string()).unwrap_or_default();
                &owned
            }
            CHDEF_CHANNEL_MAX => {
                owned = ch.max.map(|v| v.to_string()).unwrap_or_default();
                &owned
            }
            _ => return 0,
        };
        write_text(text, buf, cap)
    })
}

/// State the byte order of every multi-byte channel: [`CHDEF_LITTLE`] or
/// [`CHDEF_BIG`].
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_set_endian(handle: *mut ChdefLayout, endian: i32) -> i32 {
    guard(|| {
        let Some(layout) = layout_mut(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        layout.endian = match endian {
            CHDEF_LITTLE => Endian::Little,
            CHDEF_BIG => Endian::Big,
            _ => return CHDEF_ERR_INDEX,
        };
        CHDEF_OK
    })
}

/// State the maximum byte count of the data part, for
/// [`chdef_layout_check_capacity`].
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_set_capacity(handle: *mut ChdefLayout, capacity: u64) -> i32 {
    guard(|| {
        let Some(layout) = layout_mut(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        layout.capacity = Some(capacity as usize);
        CHDEF_OK
    })
}

/// Check the frame against the capacity stated with
/// [`chdef_layout_set_capacity`]. `out_issues` receives a handle holding
/// the finding, or an empty list when the frame fits or no capacity was
/// stated.
///
/// # Safety
///
/// The handle must be null or one of ours, and `out_issues` writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_check_capacity(
    handle: *const ChdefLayout,
    out_issues: *mut *mut ChdefIssues,
) -> i32 {
    guard(|| {
        let Some(layout) = layout_of(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        if out_issues.is_null() {
            return CHDEF_ERR_NULL;
        }
        *out_issues = into_issues(layout.check_capacity().into_iter().collect());
        CHDEF_OK
    })
}

/// Build a frame from `values` (`docs/spec/conversion.md` §5). Channels
/// `values` does not name take their default.
///
/// `out_len` always receives the frame's length, so a caller given
/// [`CHDEF_ERR_BUFFER`] learns how much room the frame needs and nothing
/// is written. `out_issues` receives the findings — a value naming an
/// unknown channel, one that is not a finite number, or a raw value wider
/// than its channel.
///
/// # Safety
///
/// The handle must be null or one of ours; `values` must be valid for
/// `value_count`, `frame` writable for `frame_cap`, and the out-params
/// writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_encode(
    handle: *const ChdefLayout,
    values: *const ChdefValue,
    value_count: usize,
    frame: *mut u8,
    frame_cap: usize,
    out_len: *mut usize,
    out_issues: *mut *mut ChdefIssues,
) -> i32 {
    guard(|| {
        let Some(layout) = layout_of(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        if out_len.is_null() || out_issues.is_null() {
            return CHDEF_ERR_NULL;
        }
        *out_issues = std::ptr::null_mut();
        *out_len = layout.total_bytes();
        if *out_len > frame_cap || frame.is_null() {
            return CHDEF_ERR_BUFFER;
        }

        let given: Vec<(u32, Value)> = if values.is_null() || value_count == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(values, value_count)
                .iter()
                .map(|v| (v.channel, v.into_value()))
                .collect()
        };

        let encoded = layout.encode(&given);
        *out_len = encoded.value.len();
        std::ptr::copy_nonoverlapping(encoded.value.as_ptr(), frame, encoded.value.len());
        *out_issues = into_issues(encoded.issues);
        CHDEF_OK
    })
}

/// Read a frame (`docs/spec/conversion.md` §6) into `out`, one entry per
/// channel that fits. A channel that overruns a short frame is omitted,
/// and so is everything after it.
///
/// `out_count` receives how many entries the frame yields, so a caller
/// given [`CHDEF_ERR_BUFFER`] learns the size to provide.
///
/// # Safety
///
/// The handle must be null or one of ours; `frame` must be valid for
/// `frame_len`, `out` writable for `out_cap`, and `out_count` writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_decode(
    handle: *const ChdefLayout,
    frame: *const u8,
    frame_len: usize,
    out: *mut ChdefReading,
    out_cap: usize,
    out_count: *mut usize,
) -> i32 {
    guard(|| {
        let Some(layout) = layout_of(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        if out_count.is_null() || (frame.is_null() && frame_len > 0) {
            return CHDEF_ERR_NULL;
        }
        let bytes = if frame.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts(frame, frame_len)
        };

        let decoded = layout.decode(bytes);
        *out_count = decoded.len();
        if decoded.len() > out_cap || out.is_null() {
            return CHDEF_ERR_BUFFER;
        }
        for (i, reading) in decoded.iter().enumerate() {
            *out.add(i) = ChdefReading {
                channel: reading.channel.number,
                raw: reading.raw,
                value: reading.value,
            };
        }
        CHDEF_OK
    })
}

/// How many diagnostics the list holds, or `0` for an unusable handle.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_issue_count(handle: *const ChdefIssues) -> u64 {
    guard_len(|| issues_of(handle).map(|i| i.len()).unwrap_or(0)) as u64
}

/// Describe the diagnostic at `index` in numbers.
///
/// # Safety
///
/// The handle must be null or one of ours, and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_issue_at(
    handle: *const ChdefIssues,
    index: usize,
    out: *mut ChdefIssue,
) -> i32 {
    guard(|| {
        let Some(issues) = issues_of(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        if out.is_null() {
            return CHDEF_ERR_NULL;
        }
        let Some(issue) = issues.get(index) else {
            return CHDEF_ERR_INDEX;
        };
        let absent_or = |v: Option<usize>| v.map(|v| v as i64).unwrap_or(-1);
        *out = ChdefIssue {
            row: absent_or(issue.row),
            col: absent_or(issue.col),
            channel: issue.channel.map(i64::from).unwrap_or(-1),
            bit: issue.bit.map(i64::from).unwrap_or(-1),
        };
        CHDEF_OK
    })
}

/// Write one of a diagnostic's text fields into `buf`; see
/// [`CHDEF_ISSUE_CODE`] and the constants beside it. Returns the length
/// the value needs, or `0` for an unusable handle, index or field.
///
/// # Safety
///
/// The handle must be null or one of ours, and `buf` writable for `cap`.
#[no_mangle]
pub unsafe extern "C" fn chdef_issue_text(
    handle: *const ChdefIssues,
    index: usize,
    field: i32,
    buf: *mut c_char,
    cap: usize,
) -> usize {
    guard_len(|| {
        let Some(issues) = issues_of(handle) else {
            return 0;
        };
        let Some(issue) = issues.get(index) else {
            return 0;
        };
        let text: &str = match field {
            CHDEF_ISSUE_CODE => issue.code.as_str(),
            CHDEF_ISSUE_FOUND => issue.found.as_deref().unwrap_or(""),
            CHDEF_ISSUE_USED => issue.used.as_deref().unwrap_or(""),
            CHDEF_ISSUE_MESSAGE => &issue.message,
            _ => return 0,
        };
        write_text(text, buf, cap)
    })
}

/// Keep the `IssueCode` import honest: the codes cross as strings, and
/// this is where that is stated once.
#[allow(dead_code)]
fn code_crosses_as_a_string(code: IssueCode) -> &'static str {
    code.as_str()
}

// -------------------------------------------------------------------- bits

/// Locate one bit of one channel: the `bit_index`th bit the BF definitions
/// name for the channel at `channel_index`, which is how far
/// [`ChdefChannel::bit_count`] counts.
fn bit_of(layout: &ChannelLayout, channel_index: usize, bit_index: usize) -> Option<&BitFieldDef> {
    let channel = layout.channels.get(channel_index)?;
    layout
        .bitfields
        .iter()
        .filter(|bit| bit.parent_channel == channel.number)
        .nth(bit_index)
}

/// Describe one named bit of one channel in numbers.
///
/// # Safety
///
/// The handle must be null or one of ours, and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_bit_at(
    handle: *const ChdefLayout,
    channel_index: usize,
    bit_index: usize,
    out: *mut ChdefBit,
) -> i32 {
    guard(|| {
        let Some(layout) = layout_of(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        if out.is_null() {
            return CHDEF_ERR_NULL;
        }
        let Some(bit) = bit_of(layout, channel_index, bit_index) else {
            return CHDEF_ERR_INDEX;
        };
        *out = ChdefBit {
            channel: bit.parent_channel,
            bit: u32::from(bit.bit_number),
            default_value: bit.default_value.map(i32::from).unwrap_or(-1),
        };
        CHDEF_OK
    })
}

/// Write one of a bit text fields into `buf`; see [`CHDEF_BIT_NAME`] and
/// the constant beside it. Returns the length the value needs, or `0` for
/// an unusable handle, index or field.
///
/// # Safety
///
/// The handle must be null or one of ours, and `buf` writable for `cap`.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_bit_text(
    handle: *const ChdefLayout,
    channel_index: usize,
    bit_index: usize,
    field: i32,
    buf: *mut c_char,
    cap: usize,
) -> usize {
    guard_len(|| {
        let Some(layout) = layout_of(handle) else {
            return 0;
        };
        let Some(bit) = bit_of(layout, channel_index, bit_index) else {
            return 0;
        };
        let text: &str = match field {
            CHDEF_BIT_NAME => &bit.name,
            CHDEF_BIT_MEMO => &bit.memo,
            _ => return 0,
        };
        write_text(text, buf, cap)
    })
}

/// How many bit readings a whole frame yields — the size to give
/// [`chdef_decode_bits`]. Bits whose parent channel the layout does not
/// hold are not among them; they are reported as diagnostics instead.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_bit_total(handle: *const ChdefLayout) -> u64 {
    guard_len(|| {
        layout_of(handle)
            .map(|layout| {
                layout
                    .channels
                    .iter()
                    .map(|ch| {
                        layout
                            .bitfields
                            .iter()
                            .filter(|bit| bit.parent_channel == ch.number)
                            .count()
                    })
                    .sum()
            })
            .unwrap_or(0)
    }) as u64
}

/// Read every named bit of a frame in one pass
/// (`docs/spec/conversion.md` §6), channel by channel and, within a
/// channel, in definition order. A channel that overruns a short frame
/// contributes no bits, and neither does anything after it.
///
/// `out_count` receives how many the frame yields, so a caller given
/// [`CHDEF_ERR_BUFFER`] learns the size to provide.
///
/// # Safety
///
/// The handle must be null or one of ours; `frame` must be valid for
/// `frame_len`, `out` writable for `out_cap`, and `out_count` writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_decode_bits(
    handle: *const ChdefLayout,
    frame: *const u8,
    frame_len: usize,
    out: *mut ChdefBitReading,
    out_cap: usize,
    out_count: *mut usize,
) -> i32 {
    guard(|| {
        let Some(layout) = layout_of(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        if out_count.is_null() || (frame.is_null() && frame_len > 0) {
            return CHDEF_ERR_NULL;
        }
        let bytes = if frame.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts(frame, frame_len)
        };

        let decoded = layout.decode(bytes);
        let readings: Vec<ChdefBitReading> = decoded
            .iter()
            .flat_map(|reading| {
                reading.bits().map(|(bit, set)| ChdefBitReading {
                    channel: bit.parent_channel,
                    bit: u32::from(bit.bit_number),
                    value: i32::from(set),
                })
            })
            .collect();

        *out_count = readings.len();
        if readings.len() > out_cap || out.is_null() {
            return CHDEF_ERR_BUFFER;
        }
        for (index, reading) in readings.iter().enumerate() {
            *out.add(index) = *reading;
        }
        CHDEF_OK
    })
}

// ---------------------------------------------------------------- a reading

/// Which of the two readings the channel `format` column selects
/// (`docs/spec/conversion.md` §7), as a value carrying its own notation.
///
/// # Safety
///
/// The handle must be null or one of ours, and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_channel_displayed(
    handle: *const ChdefLayout,
    index: usize,
    raw: u64,
    out: *mut ChdefValue,
) -> i32 {
    guard(|| {
        let Some(layout) = layout_of(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        if out.is_null() {
            return CHDEF_ERR_NULL;
        }
        let Some(ch) = layout.channels.get(index) else {
            return CHDEF_ERR_INDEX;
        };
        *out = match ch.displayed_value(raw) {
            Value::Physical(v) => ChdefValue::physical(ch.number, v),
            Value::Raw(v) => ChdefValue::raw(ch.number, v),
        };
        CHDEF_OK
    })
}

/// The default text form of a reading (`docs/spec/conversion.md` §7),
/// which a consumer with its own digit counts or separators replaces.
/// Returns the length the text needs, or `0` for an unusable handle or
/// index.
///
/// # Safety
///
/// The handle must be null or one of ours, and `buf` writable for `cap`.
#[no_mangle]
pub unsafe extern "C" fn chdef_layout_channel_render(
    handle: *const ChdefLayout,
    index: usize,
    raw: u64,
    buf: *mut c_char,
    cap: usize,
) -> usize {
    guard_len(|| {
        let Some(layout) = layout_of(handle) else {
            return 0;
        };
        let Some(ch) = layout.channels.get(index) else {
            return 0;
        };
        write_text(&ch.render(raw), buf, cap)
    })
}

// ----------------------------------------------------------- the notation

/// Read the text form of a value (`docs/spec/format.md` §3): a leading
/// `0x` or `0X` is a raw bit pattern, anything else a physical value.
/// `channel` is stamped onto the result so it can be handed straight to
/// [`chdef_encode`]. Text that denotes no value is [`CHDEF_ERR_VALUE`].
///
/// # Safety
///
/// `text` must be valid for `len`, and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn chdef_value_parse(
    text: *const u8,
    len: usize,
    channel: u32,
    out: *mut ChdefValue,
) -> i32 {
    guard(|| {
        if out.is_null() {
            return CHDEF_ERR_NULL;
        }
        let source = match borrow_str(text, len) {
            None => return CHDEF_ERR_NULL,
            Some(Err(())) => return CHDEF_ERR_UTF8,
            Some(Ok(source)) => source,
        };
        let Some(value) = Value::parse(source) else {
            return CHDEF_ERR_VALUE;
        };
        *out = match value {
            Value::Physical(v) => ChdefValue::physical(channel, v),
            Value::Raw(v) => ChdefValue::raw(channel, v),
        };
        CHDEF_OK
    })
}

// -------------------------------------------------------------------- grid

/// Read definition bytes as cells (`docs/spec/editing.md`), whatever
/// columns they name. On success `out_grid` receives a handle the caller
/// frees with [`chdef_grid_free`]; on failure it is left null and, when
/// `err_buf` is not null, the error message is written into it.
///
/// # Safety
///
/// `bytes` must be valid for `len`, `out_grid` writable, and `err_buf`
/// writable for `err_cap`.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_parse(
    bytes: *const u8,
    len: usize,
    out_grid: *mut *mut ChdefGrid,
    err_buf: *mut c_char,
    err_cap: usize,
) -> i32 {
    guard(|| {
        if out_grid.is_null() {
            return CHDEF_ERR_NULL;
        }
        *out_grid = std::ptr::null_mut();
        if bytes.is_null() && len > 0 {
            return CHDEF_ERR_NULL;
        }
        let source = if bytes.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts(bytes, len)
        };
        match Grid::parse_bytes(source) {
            Ok(grid) => {
                *out_grid = Box::into_raw(Box::new(ChdefGrid {
                    tag: GRID_TAG,
                    grid,
                }));
                CHDEF_OK
            }
            Err(e) => {
                write_text(&e.to_string(), err_buf, err_cap);
                status_of(&e)
            }
        }
    })
}

/// Release a grid. A null handle is ignored.
///
/// # Safety
///
/// The handle must come from [`chdef_grid_parse`] and must not have been
/// freed already.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_free(handle: *mut ChdefGrid) {
    guard(|| {
        if let Some(h) = handle.as_mut() {
            if h.tag == GRID_TAG {
                h.tag = 0;
                drop(Box::from_raw(handle));
            }
        }
        CHDEF_OK
    });
}

/// How many data rows the grid has, comment and blank rows included, or
/// `0` for an unusable handle.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_row_count(handle: *const ChdefGrid) -> u64 {
    guard_len(|| grid_of(handle).map(Grid::row_count).unwrap_or(0)) as u64
}

/// How many cells the header row has, or `0` when the file was read
/// without one. A record always holds at least one cell, so `0` is not a
/// header of no columns.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_header_count(handle: *const ChdefGrid) -> u64 {
    guard_len(|| {
        grid_of(handle)
            .and_then(Grid::header)
            .map(<[String]>::len)
            .unwrap_or(0)
    }) as u64
}

/// How many cells the data row at `row` has, or `0` outside the grid.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_col_count(handle: *const ChdefGrid, row: usize) -> u64 {
    guard_len(|| {
        grid_of(handle)
            .and_then(|grid| grid.row(row))
            .map(<[String]>::len)
            .unwrap_or(0)
    }) as u64
}

/// Write one header cell into `buf`. Returns the length it needs, or `0`
/// for an unusable handle or index.
///
/// # Safety
///
/// The handle must be null or one of ours, and `buf` writable for `cap`.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_header_at(
    handle: *const ChdefGrid,
    col: usize,
    buf: *mut c_char,
    cap: usize,
) -> usize {
    guard_len(|| {
        let text = grid_of(handle)
            .and_then(Grid::header)
            .and_then(|header| header.get(col))
            .map(String::as_str)
            .unwrap_or("");
        write_text(text, buf, cap)
    })
}

/// Write one data cell into `buf`, 0-based with the header excluded — the
/// row numbering Issues use. Returns the length it needs, or `0` outside
/// the grid.
///
/// # Safety
///
/// The handle must be null or one of ours, and `buf` writable for `cap`.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_cell(
    handle: *const ChdefGrid,
    row: usize,
    col: usize,
    buf: *mut c_char,
    cap: usize,
) -> usize {
    guard_len(|| {
        let text = grid_of(handle)
            .and_then(|grid| grid.cell(row, col))
            .unwrap_or("");
        write_text(text, buf, cap)
    })
}

/// Overwrite one data cell; a row shorter than `col` is padded with empty
/// cells. A row outside the grid is [`CHDEF_ERR_INDEX`].
///
/// # Safety
///
/// The handle must be null or one of ours, and `value` valid for `len`.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_set_cell(
    handle: *mut ChdefGrid,
    row: usize,
    col: usize,
    value: *const u8,
    len: usize,
) -> i32 {
    guard(|| {
        let Some(grid) = grid_mut(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        let text = match borrow_str(value, len) {
            None => return CHDEF_ERR_NULL,
            Some(Err(())) => return CHDEF_ERR_UTF8,
            Some(Ok(text)) => text,
        };
        if grid.row(row).is_none() {
            return CHDEF_ERR_INDEX;
        }
        grid.set_cell(row, col, text);
        CHDEF_OK
    })
}

/// Insert an empty data row at `at`, clamped to the end. Cells are written
/// with [`chdef_grid_set_cell`], which pads a short row.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_insert_row(handle: *mut ChdefGrid, at: usize) -> i32 {
    guard(|| {
        let Some(grid) = grid_mut(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        grid.insert_row(at, Vec::new());
        CHDEF_OK
    })
}

/// Append an empty data row after the last.
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_append_row(handle: *mut ChdefGrid) -> i32 {
    guard(|| {
        let Some(grid) = grid_mut(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        grid.append_row(Vec::new());
        CHDEF_OK
    })
}

/// Remove the data row at `at`. A row outside the grid removes nothing and
/// is [`CHDEF_ERR_INDEX`].
///
/// # Safety
///
/// The handle must be null or one of ours.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_remove_row(handle: *mut ChdefGrid, at: usize) -> i32 {
    guard(|| {
        let Some(grid) = grid_mut(handle) else {
            return CHDEF_ERR_HANDLE;
        };
        match grid.remove_row(at) {
            Some(_) => CHDEF_OK,
            None => CHDEF_ERR_INDEX,
        }
    })
}

/// Write the file back into `buf`, in the shape it was read in — its
/// byte-order mark and record separator (`docs/spec/editing.md` §2).
/// Returns the length it needs, or `0` for an unusable handle.
///
/// # Safety
///
/// The handle must be null or one of ours, and `buf` writable for `cap`.
#[no_mangle]
pub unsafe extern "C" fn chdef_grid_to_csv(
    handle: *const ChdefGrid,
    buf: *mut c_char,
    cap: usize,
) -> usize {
    guard_len(|| {
        let Some(grid) = grid_of(handle) else {
            return 0;
        };
        write_text(&grid.to_csv(), buf, cap)
    })
}
