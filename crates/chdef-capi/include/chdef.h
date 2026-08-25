/* chdef — a C ABI over the chdef crate: read CH / BF definitions, describe
 * the frame layout, encode and decode frames, read the named bits of a
 * channel, and edit a definition file as cells.
 *
 * SPDX-License-Identifier: MIT OR Apache-2.0
 *
 * What crosses is every rule docs/spec states, so that a consumer never
 * reimplements one (ADR-0023). What does not cross is what a consumer
 * writes anyway: editing UI, undo history, save orchestration.
 *
 * Three rules shape every signature (ADR-0021):
 *
 *  - No enums. Every identity crosses as its stable ASCII string — an
 *    Issue's code, a channel's type — because both vocabularies are
 *    documented as growing. `endian` is the one exception: two values that
 *    cannot grow.
 *  - Strings go into the caller's buffer. A `_text` call writes UTF-8 into
 *    `buf`, always NUL-terminating, and returns the length the value needs
 *    (excluding the terminator). Call it with `buf == NULL, cap == 0` to
 *    ask the length first. Nothing chdef produces is the caller's to free.
 *  - Absent numbers are -1. `row`, `col`, `channel`, `bit` and a channel's
 *    `default_value` are non-negative when they exist.
 *
 * Every entry point catches Rust panics and reports CHDEF_PANIC.
 *
 * Handles come from chdef_layout_parse / chdef_grid_parse /
 * chdef_vocabulary_new and are released exactly once with the matching
 * free. Passing NULL to a free is a no-op, and passing a handle of one
 * kind where another is expected is reported as CHDEF_ERR_HANDLE rather
 * than read as the wrong type.
 *
 * Using a handle after freeing it is UNDEFINED, as for any C pointer. The
 * tag is cleared before the memory is released, so a stale handle is often
 * caught, but the memory belongs to the allocator by then and that is not
 * a guarantee.
 */

#ifndef CHDEF_H
#define CHDEF_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The revision of this ABI. It rises whenever a symbol is added or
 * changed; check that chdef_abi_version() is at least this value before
 * calling anything else. Symbols are added and never withdrawn, so a newer
 * library serves an older caller. */
#define CHDEF_ABI_VERSION 6u

/* Statuses. */
#define CHDEF_OK 0
#define CHDEF_ERR_HANDLE (-1)
#define CHDEF_ERR_INDEX (-2)
#define CHDEF_ERR_BUFFER (-3)
#define CHDEF_ERR_NULL (-4)
#define CHDEF_ERR_UTF8 (-5)
#define CHDEF_ERR_CSV (-6)
#define CHDEF_ERR_IO (-7)
#define CHDEF_ERR_VALUE (-8) /* the text denotes no value */
#define CHDEF_ERR_COLUMN (-9) /* no column answers to that name */
#define CHDEF_PANIC (-99)

/* Byte order, for chdef_layout_set_endian. */
#define CHDEF_LITTLE 0
#define CHDEF_BIG 1

/* Text fields of a channel, for chdef_layout_channel_text. */
#define CHDEF_CHANNEL_NAME 0
#define CHDEF_CHANNEL_TYPE 1    /* "UI" / "SI" / "BF" */
#define CHDEF_CHANNEL_UNIT 2
#define CHDEF_CHANNEL_SECTION 3
#define CHDEF_CHANNEL_MEMO 4
#define CHDEF_CHANNEL_VAR 5
#define CHDEF_CHANNEL_FORMAT 6  /* "DEC" / "HEX" */
#define CHDEF_CHANNEL_MIN 7     /* in its cell's notation; "" when unstated */
#define CHDEF_CHANNEL_MAX 8
#define CHDEF_CHANNEL_KIND 9  /* "plain" / "const" / "counter" / "derived" */
#define CHDEF_CHANNEL_DERIVED 10 /* the recipe cell; "" when not derived */

/* Text fields of a diagnostic, for chdef_issue_text. */
#define CHDEF_ISSUE_CODE 0      /* the stable ASCII identifier */
#define CHDEF_ISSUE_FOUND 1     /* the value chdef could not use */
#define CHDEF_ISSUE_USED 2      /* the value it used instead */
#define CHDEF_ISSUE_MESSAGE 3   /* English prose; wording is not contracted */

/* Text fields of a named bit, for chdef_layout_bit_text. */
#define CHDEF_BIT_NAME 0
#define CHDEF_BIT_MEMO 1

/* Which CSV a column belongs to, for chdef_column_name and
 * chdef_vocabulary_teach. */
#define CHDEF_COLUMNS_CH 0
#define CHDEF_COLUMNS_BF 1

/* An opaque frame layout. */
typedef struct ChdefLayout ChdefLayout;
/* An opaque list of diagnostics. */
typedef struct ChdefIssues ChdefIssues;
/* An opaque grid of cells. */
typedef struct ChdefGrid ChdefGrid;
/* An opaque column vocabulary. */
typedef struct ChdefVocabulary ChdefVocabulary;

/* One channel of the layout. `default_value` is -1 when the channel states
 * none; `bit_count` is how many named bits it has. */
typedef struct ChdefChannel {
  uint32_t number;
  uint64_t at;
  uint64_t bytes;
  double lsb;
  double offset;
  int64_t default_value;
  int32_t favorite;
  uint64_t bit_count;
} ChdefChannel;

/* One diagnostic. Fields are -1 when the finding carries none. */
typedef struct ChdefIssue {
  int64_t row;
  int64_t col;
  int64_t channel;
  int64_t bit;
} ChdefIssue;

/* A value handed to chdef_encode. `kind` is 0 to take `physical`, 1 to
 * take `raw`. */
typedef struct ChdefValue {
  uint32_t channel;
  int32_t kind;
  double physical;
  uint64_t raw;
} ChdefValue;

/* One channel's readings from a decoded frame. */
typedef struct ChdefReading {
  uint32_t channel;
  uint64_t raw;
  double value;
} ChdefReading;

/* One named bit of a channel. `default_value` is 0 or 1, or -1 when the BF
 * row names none and the bit keeps the parent channel's. */
typedef struct ChdefBit {
  uint32_t channel;
  uint32_t bit;
  int32_t default_value;
} ChdefBit;

/* One named bit of a decoded frame, and whether it is set. */
typedef struct ChdefBitReading {
  uint32_t channel;
  uint32_t bit;
  int32_t value;
} ChdefBitReading;

uint32_t chdef_abi_version(void);

/* Read a CH CSV and an optional BF CSV (UTF-8, `bf` may be NULL with
 * bf_len 0) into a layout. On CHDEF_OK both out-params receive handles the
 * caller frees. On any other status both are left NULL and, when err_buf
 * is not NULL, the error's message is written into it. */
int32_t chdef_layout_parse(const uint8_t *ch, size_t ch_len,
                           const uint8_t *bf, size_t bf_len,
                           ChdefLayout **out_layout,
                           ChdefIssues **out_issues,
                           char *err_buf, size_t err_cap);

void chdef_layout_free(ChdefLayout *handle);
void chdef_issues_free(ChdefIssues *handle);

/* The canonical column names, in canonical order: the identity of each
 * column, the names a vocabulary is taught against, and the keys the JSON
 * of the interchange format uses. A column crosses as its name rather than
 * as a number, so adding one is not an ABI break. */
uint64_t chdef_column_count(int32_t kind);
size_t chdef_column_name(int32_t kind, size_t index, char *buf, size_t cap);

/* A column vocabulary: the spellings one caller accepts for the columns,
 * and the spelling it writes for each. A vocabulary is data, not a
 * language chdef knows — chdef_vocabulary_japanese is one value among any
 * number a caller can build, and has no standing they lack.
 *
 * chdef_vocabulary_teach takes a canonical column name (see
 * chdef_column_name); a name no column answers to is CHDEF_ERR_COLUMN. The
 * FIRST spelling taught for a column is the one written for a file created
 * with this vocabulary. */
int32_t chdef_vocabulary_new(ChdefVocabulary **out);
int32_t chdef_vocabulary_japanese(ChdefVocabulary **out);
void chdef_vocabulary_free(ChdefVocabulary *handle);
int32_t chdef_vocabulary_teach(ChdefVocabulary *handle, int32_t kind,
                               const uint8_t *spelling, size_t spelling_len,
                               const uint8_t *column, size_t column_len);

/* chdef_layout_parse, reading both headers with `vocabulary` on top of the
 * canonical names. A NULL vocabulary is the empty one. */
int32_t chdef_layout_parse_with(const uint8_t *ch, size_t ch_len,
                                const uint8_t *bf, size_t bf_len,
                                const ChdefVocabulary *vocabulary,
                                ChdefLayout **out_layout,
                                ChdefIssues **out_issues,
                                char *err_buf, size_t err_cap);

/* The data length of the frame in bytes, and how many channels it has.
 * Both are 0 for an unusable handle. */
uint64_t chdef_layout_total_bytes(const ChdefLayout *handle);
uint64_t chdef_layout_channel_count(const ChdefLayout *handle);

int32_t chdef_layout_channel_at(const ChdefLayout *handle, size_t index,
                                ChdefChannel *out);
size_t chdef_layout_channel_text(const ChdefLayout *handle, size_t index,
                                 int32_t field, char *buf, size_t cap);

int32_t chdef_layout_set_endian(ChdefLayout *handle, int32_t endian);
int32_t chdef_layout_set_capacity(ChdefLayout *handle, uint64_t capacity);
/* The maximum number of channels the port accepts — the limit a byte
 * count cannot express. Both limits are reported by limits_exceeded. */
int32_t chdef_layout_set_channel_capacity(ChdefLayout *handle,
                                          uint64_t channels);
/* Check the frame against the capacity stated with set_capacity;
 * out_issues receives an empty list when it fits or none was stated. */
int32_t chdef_layout_limits_exceeded(const ChdefLayout *handle,
                                    ChdefIssues **out_issues);

/* Build a frame from `values`; channels not named take their default.
 * out_len always receives the frame's length, so a caller given
 * CHDEF_ERR_BUFFER learns the room it needs and nothing is written. */
int32_t chdef_encode(const ChdefLayout *handle, const ChdefValue *values,
                     size_t value_count, uint8_t *frame, size_t frame_cap,
                     size_t *out_len, ChdefIssues **out_issues);

/* Read a frame into `out`, one entry per channel that fits. out_count
 * always receives how many entries the frame yields. */
int32_t chdef_decode(const ChdefLayout *handle, const uint8_t *frame,
                     size_t frame_len, ChdefReading *out, size_t out_cap,
                     size_t *out_count);

/* The named bits of one channel: bit_index runs to that channel's
 * bit_count. Out of range is CHDEF_ERR_INDEX. */
int32_t chdef_layout_bit_at(const ChdefLayout *handle, size_t channel_index,
                            size_t bit_index, ChdefBit *out);
size_t chdef_layout_bit_text(const ChdefLayout *handle, size_t channel_index,
                             size_t bit_index, int32_t field, char *buf,
                             size_t cap);

/* How many bit readings a whole frame yields — the size to give
 * chdef_decode_bits. */
uint64_t chdef_layout_bit_total(const ChdefLayout *handle);

/* Read every named bit of a frame in one pass, channel by channel and, in
 * a channel, in definition order. out_count always receives how many the
 * frame yields. */
int32_t chdef_decode_bits(const ChdefLayout *handle, const uint8_t *frame,
                          size_t frame_len, ChdefBitReading *out,
                          size_t out_cap, size_t *out_count);

/* Which of the two readings the channel's `format` column selects, and the
 * default text form of it. */
int32_t chdef_layout_channel_displayed(const ChdefLayout *handle,
                                       size_t index, uint64_t raw,
                                       ChdefValue *out);
size_t chdef_layout_channel_render(const ChdefLayout *handle, size_t index,
                                   uint64_t raw, char *buf, size_t cap);

/* Read the text form of a value: a leading "0x" / "0X" is a raw bit
 * pattern, anything else a physical value. `channel` is stamped onto the
 * result so it can be handed straight to chdef_encode. Text that denotes
 * no value is CHDEF_ERR_VALUE. */
int32_t chdef_value_parse(const uint8_t *text, size_t len, uint32_t channel,
                          ChdefValue *out);

/* Read definition bytes as cells, whatever columns they name. On CHDEF_OK
 * out_grid receives a handle the caller frees; on any other status it is
 * left NULL and the error's message is written into err_buf. */
int32_t chdef_grid_parse(const uint8_t *bytes, size_t len,
                         ChdefGrid **out_grid, char *err_buf,
                         size_t err_cap);
void chdef_grid_free(ChdefGrid *handle);

/* Row and column counts. header_count is 0 when the file was read without
 * a header; a record always holds at least one cell, so 0 is never a
 * header of no columns. */
uint64_t chdef_grid_row_count(const ChdefGrid *handle);
uint64_t chdef_grid_header_count(const ChdefGrid *handle);
uint64_t chdef_grid_col_count(const ChdefGrid *handle, size_t row);

/* Cells. Data rows are 0-based with the header excluded — the row
 * numbering diagnostics use. */
size_t chdef_grid_header_at(const ChdefGrid *handle, size_t col, char *buf,
                            size_t cap);
size_t chdef_grid_cell(const ChdefGrid *handle, size_t row, size_t col,
                       char *buf, size_t cap);
int32_t chdef_grid_set_cell(ChdefGrid *handle, size_t row, size_t col,
                            const uint8_t *value, size_t len);

/* Rows are inserted empty and filled with chdef_grid_set_cell, which pads
 * a short row. insert clamps to the end; removing a row outside the grid
 * removes nothing and is CHDEF_ERR_INDEX. */
int32_t chdef_grid_insert_row(ChdefGrid *handle, size_t at);
int32_t chdef_grid_append_row(ChdefGrid *handle);
int32_t chdef_grid_remove_row(ChdefGrid *handle, size_t at);

/* Write the file back in the shape it was read in — its byte-order mark
 * and record separator. */
size_t chdef_grid_to_csv(const ChdefGrid *handle, char *buf, size_t cap);

/* Derived channels: the recipes this library knows by name. The set can
 * grow, and a recipe naming something outside it is still read — its
 * coverage is available through chdef_covered_bytes. */
uint64_t chdef_recipe_count(void);
size_t chdef_recipe_name(size_t index, char *buf, size_t cap);

/* Fill every derived channel of `frame`. chdef_encode never does this:
 * sealing is a call of its own, made once after every other value is in
 * place. Nothing is written for a channel that is reported. */
int32_t chdef_seal(const ChdefLayout *handle, uint8_t *frame,
                   size_t frame_len, ChdefIssues **out_issues);

/* Which derived channels disagree with their recipe — the check a
 * receiver makes. Nothing is changed. */
int32_t chdef_derived_mismatches(const ChdefLayout *handle,
                                 const uint8_t *frame, size_t frame_len,
                                 ChdefIssues **out_issues);

/* The bytes a derived channel's recipe covers, in the order it covers
 * them — the storey below chdef_seal. A device whose checksum chdef does
 * not compute still says which bytes it covers, so a caller runs its own
 * over exactly those and writes the result through chdef_encode. out_len
 * always receives the length the coverage needs. */
int32_t chdef_covered_bytes(const ChdefLayout *handle, uint32_t channel,
                            const uint8_t *frame, size_t frame_len,
                            uint8_t *out, size_t out_cap, size_t *out_len);

/* Which values fall outside their channel's declared min / max. Nothing
 * is changed and nothing is remembered: encode and decode behave the same
 * whether these were called or not. chdef_readings_out_of_range reads only the
 * channel and value of each reading. */
int32_t chdef_values_out_of_range(const ChdefLayout *handle,
                           const ChdefValue *values, size_t value_count,
                           ChdefIssues **out_issues);
int32_t chdef_readings_out_of_range(const ChdefLayout *handle,
                             const ChdefReading *readings,
                             size_t reading_count,
                             ChdefIssues **out_issues);

/* Every Issue code this library can report. The codes cross as strings
 * so the vocabulary can grow, and this is what lets a caller prove its
 * own table covers them. */
uint64_t chdef_issue_code_count(void);
size_t chdef_issue_code_name(size_t index, char *buf, size_t cap);

uint64_t chdef_issue_count(const ChdefIssues *handle);
int32_t chdef_issue_at(const ChdefIssues *handle, size_t index,
                       ChdefIssue *out);
size_t chdef_issue_text(const ChdefIssues *handle, size_t index,
                        int32_t field, char *buf, size_t cap);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* CHDEF_H */
