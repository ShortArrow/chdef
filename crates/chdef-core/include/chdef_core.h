/* chdef-core — the raw-only rules of chdef for a device: where each channel
 * sits, its raw value out of a frame and into one, every channel's default,
 * and the CRC a derived channel computes over the bytes it covers.
 *
 * SPDX-License-Identifier: MIT OR Apache-2.0
 *
 * A device does not read a definition file. It holds the layout the file
 * describes, fixed when the firmware was built (ADR-0034), and `chdef-gen`
 * writes that layout out as the CHDEF_LAYOUT these calls take. Names,
 * units, `lsb`, `offset` and the diagnostics stay on the host: the target
 * is raw-only, and a device that scales does so in its own code.
 *
 * The rules have one implementation, in the Rust crate this header
 * declares. Link it as a static library built for the firmware's target;
 * there is no generated codec to diverge from it (docs/spec/embedded.md).
 *
 * Every call returns 1 on success and 0 when the frame is shorter than the
 * layout, the channel is not in it, or a pointer is NULL. No other status
 * exists: the definition was checked when the table was generated, and a
 * device has nowhere to report one.
 *
 * The tables a call is given must outlive it, and nothing here allocates,
 * frees or keeps a pointer past the call that was handed it.
 */

#ifndef CHDEF_CORE_H
#define CHDEF_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The two byte orders a frame can take. */
#define CHDEF_CORE_LITTLE 0
#define CHDEF_CORE_BIG 1

/* One channel of the layout: where it sits, how wide it is, and the value a
 * frame starts from. `bytes` is 1..=8; a wider value counts as 8 and 0 as 1.
 * `default_value` is the channel's raw default with its BF rows already
 * merged in — C cannot name a field `default`, which is what the Rust field
 * is called, so the name differs here and the order does not. */
typedef struct ChdefCoreSlot {
  uint32_t number;
  uint32_t at;
  uint8_t bytes;
  uint64_t default_value;
} ChdefCoreSlot;

/* The Rocksoft model: the six numbers that decide a CRC completely. `refin`
 * reflects each input byte before it enters the register and `refout`
 * reflects the register at the end. */
typedef struct ChdefCoreCrc {
  uint32_t width;
  uint64_t poly;
  uint64_t init;
  bool refin;
  bool refout;
  uint64_t xorout;
} ChdefCoreCrc;

/* A stretch of the frame a recipe covers, resolved from channel numbers to
 * an offset and a length when the table was generated. */
typedef struct ChdefCoreRange {
  uint32_t at;
  uint32_t len;
} ChdefCoreRange;

/* A channel computed from the rest of the frame. `slot` is an index into
 * the layout's slot table, not a channel number; `covers` holds
 * `cover_count` stretches in the order the recipe wrote them. */
typedef struct ChdefCoreDerived {
  uint32_t slot;
  ChdefCoreCrc crc;
  const ChdefCoreRange *covers;
  size_t cover_count;
} ChdefCoreDerived;

/* A frame's layout as the firmware holds it. `endian` is CHDEF_CORE_LITTLE
 * (0) or CHDEF_CORE_BIG (1) and applies to the whole frame; `total` is the
 * frame's length in bytes. chdef-gen writes one of these as CHDEF_LAYOUT. */
typedef struct ChdefCoreLayout {
  const ChdefCoreSlot *slots;
  size_t slot_count;
  const ChdefCoreDerived *derived;
  size_t derived_count;
  int32_t endian;
  uint32_t total;
} ChdefCoreLayout;

/* One channel's raw value out of a frame, into *out_raw. */
int32_t chdef_core_read(const ChdefCoreLayout *layout, const uint8_t *frame,
                        size_t len, uint32_t number, uint64_t *out_raw);

/* One channel's raw value into a frame. */
int32_t chdef_core_write(const ChdefCoreLayout *layout, uint8_t *frame,
                         size_t len, uint32_t number, uint64_t raw);

/* Every channel's default into a frame. */
int32_t chdef_core_fill_defaults(const ChdefCoreLayout *layout, uint8_t *frame,
                                 size_t len);

/* Every derived channel computed and written; on 0, nothing is written. */
int32_t chdef_core_seal(const ChdefCoreLayout *layout, uint8_t *frame,
                        size_t len);

/* Whether every derived channel holds its computed value. */
int32_t chdef_core_verify(const ChdefCoreLayout *layout, const uint8_t *frame,
                          size_t len);

#ifdef __cplusplus
}
#endif

#endif /* CHDEF_CORE_H */
