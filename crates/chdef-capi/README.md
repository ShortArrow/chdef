# chdef-capi

A C ABI over [chdef](https://crates.io/crates/chdef): read CH / BF
definitions, describe the frame layout, encode and decode frames, read the
named bits of a channel, and edit a definition file as cells.

> **⚠ Pre-alpha (0.0.x).** The API and the CSV rules are still moving;
> until 0.1.0 lands, patch releases may break either.

**Calling from C#?** Do not build this — `dotnet add package Chdef` carries
the native library for every supported runtime.

## Building it

```toml
[dependencies]
chdef-capi = "0.0"

[lib]
crate-type = ["cdylib"]
```

Or build the crate itself: `cargo build -p chdef-capi --release` leaves
`libchdef_capi.so` / `.dylib` / `chdef_capi.dll` in the target directory.
The header is `include/chdef.h`, checked in and kept honest by a test that
fails if any exported symbol is missing from it.

## Using it

```c
#include "chdef.h"

/* Symbols are added and never withdrawn, so the check is one-sided. */
if (chdef_abi_version() < CHDEF_ABI_VERSION) return -1;

const char *ch = "number,bytes,type,name,lsb\n1,2,UI,speed,0.5\n";
ChdefLayout *layout = NULL;
ChdefIssues *issues = NULL;
char error[256];

if (chdef_layout_parse((const uint8_t *)ch, strlen(ch), NULL, 0,
                       &layout, &issues, error, sizeof error) != CHDEF_OK) {
    fprintf(stderr, "%s\n", error);
    return -1;
}

/* A bad row never stops the load; it arrives here instead. */
for (size_t i = 0; i < chdef_issue_count(issues); i++) {
    char code[64];
    chdef_issue_text(issues, i, CHDEF_ISSUE_CODE, code, sizeof code);
    fprintf(stderr, "%s\n", code);
}

uint8_t frame[] = { 0x40, 0x00 };
ChdefReading readings[1];
size_t count = 0;
chdef_decode(layout, frame, sizeof frame, readings, 1, &count);
printf("%f\n", readings[0].value);   /* 32.0 */

chdef_issues_free(issues);
chdef_layout_free(layout);
```

## Three rules shape every signature

- **No enumerations cross.** A data type, an Issue code, a display format
  each cross as a stable ASCII string, so adding one is not an ABI break
  and a caller that does not know it still has something to show.
- **Strings go into your buffer.** A `_text` call writes UTF-8, always
  terminates, and returns the length the value needs — call it with
  `buf == NULL, cap == 0` to ask the length first. Nothing chdef produces
  is yours to free.
- **Absent numbers are -1.** `row`, `col`, `channel`, `bit` and a
  channel's `default_value` are non-negative when they exist.

Every entry point catches a Rust panic and reports `CHDEF_PANIC`. A handle
of one kind passed where another is expected is `CHDEF_ERR_HANDLE` rather
than read as the wrong type; using a handle after freeing it is undefined,
as for any C pointer.

## Where to look next

- [What crosses this boundary](https://github.com/ShortArrow/chdef/blob/main/docs/spec/abi.md)
  — and why it is everything the specification states
- [The guide](https://github.com/ShortArrow/chdef/blob/main/docs/guide.md)
  — the shortest path through each task
- [The specification](https://github.com/ShortArrow/chdef/blob/main/docs/spec/README.md)
  — what the format is, exactly

## License

MIT OR Apache-2.0.
