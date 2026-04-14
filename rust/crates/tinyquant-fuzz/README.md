# rust/crates/tinyquant-fuzz

`tinyquant-fuzz` houses `libfuzzer-sys` fuzz targets for TinyQuant. The crate
is wired in a later phase; targets will be added as each serialization surface
is implemented. It is kept as a separate crate so fuzz builds do not affect the
normal `cargo test` / `cargo build` pipeline for the rest of the workspace.

## What lives here

| Path | Role |
|---|---|
| `src/lib.rs` | Crate root; fuzz target implementations added here as serialization surfaces land |
| `tests/smoke.rs` | Smoke test confirming the crate compiles and links cleanly |

## How this area fits the system

Fuzz targets are run independently with `cargo fuzz` (not part of the default
test suite). The `tests/smoke.rs` test runs under normal `cargo test` so CI
catches compilation failures before anyone attempts a fuzz run.

## Common edit paths

- New fuzz targets: `src/lib.rs` (one target function per serialization surface)
- Build-level sanity: `tests/smoke.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
