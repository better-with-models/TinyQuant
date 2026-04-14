# rust/crates/tinyquant-fuzz/src

Source for the `tinyquant-fuzz` crate. Currently contains only `lib.rs`;
fuzz target functions will be added here as serialization surfaces in the
codec are implemented.

## What lives here

| File | Role |
| --- | --- |
| `lib.rs` | Crate root; will contain `libfuzzer-sys` fuzz target functions |

## How this area fits the system

Each fuzz target in `lib.rs` is exercised by `cargo fuzz run <target-name>`.
The crate depends on `libfuzzer-sys` and on whichever `tinyquant-*` crates own
the serialization surfaces under test. No fuzz targets run during normal CI;
they are run on demand or in a dedicated fuzzing workflow.

## Common edit paths

- Adding a fuzz target for a new codec surface: `lib.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
