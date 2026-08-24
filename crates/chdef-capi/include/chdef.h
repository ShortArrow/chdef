/* chdef — a C ABI over the chdef crate: read CH / BF definitions, describe
 * the frame layout, encode and decode frames.
 *
 * SPDX-License-Identifier: MIT OR Apache-2.0
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
 * Handles come from chdef_layout_parse and are released exactly once with
 * chdef_layout_free / chdef_issues_free. Passing NULL to a free is a
 * no-op; passing a freed or foreign pointer to anything else is reported
 * as CHDEF_ERR_HANDLE rather than read.
 */

#ifndef CHDEF_H
#define CHDEF_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The revision of this ABI. Check chdef_abi_version() against it before
 * calling anything else. */
#define CHDEF_ABI_VERSION 1u

/* Statuses. */
#define CHDEF_OK 0
#define CHDEF_ERR_HANDLE (-1)
#define CHDEF_ERR_INDEX (-2)
#define CHDEF_ERR_BUFFER (-3)
#define CHDEF_ERR_NULL (-4)
#define CHDEF_ERR_UTF8 (-5)
#define CHDEF_ERR_CSV (-6)
#define CHDEF_ERR_IO (-7)
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

/* Text fields of a diagnostic, for chdef_issue_text. */
#define CHDEF_ISSUE_CODE 0      /* the stable ASCII identifier */
#define CHDEF_ISSUE_FOUND 1     /* the value chdef could not use */
#define CHDEF_ISSUE_USED 2      /* the value it used instead */
#define CHDEF_ISSUE_MESSAGE 3   /* English prose; wording is not contracted */

/* An opaque frame layout. */
typedef struct ChdefLayout ChdefLayout;
/* An opaque list of diagnostics. */
typedef struct ChdefIssues ChdefIssues;

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
/* Check the frame against the capacity stated with set_capacity;
 * out_issues receives an empty list when it fits or none was stated. */
int32_t chdef_layout_check_capacity(const ChdefLayout *handle,
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

uint64_t chdef_issue_count(const ChdefIssues *handle);
int32_t chdef_issue_at(const ChdefIssues *handle, size_t index,
                       ChdefIssue *out);
size_t chdef_issue_text(const ChdefIssues *handle, size_t index,
                        int32_t field, char *buf, size_t cap);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* CHDEF_H */
