# rust/xtask

This directory contains the `xtask` workspace task runner for the TinyQuant Rust workspace. Run from the `rust/` directory with `cargo xtask <command>`. It replaces ad-hoc shell scripts with a type-checked Rust binary that handles formatting, linting, testing, fixture regeneration, benchmark budget checks, CI/docs parity assertions, and publish-guard drift checks. The task runner is defined as a separate Cargo package so it can be invoked without being part of the main workspace library surface.

## What lives here

- `src/` — task runner source (see [`src/README.md`](src/README.md)).
- `Cargo.toml` — package manifest; xtask is not part of the workspace `members` list so it does not interfere with workspace-wide feature resolution.

Available top-level commands: `fmt`, `lint`, `test`, `fixtures`, `bench`, `bench-budget`, `check-matrix-sync`, `check-publish-guards`, `docs`, `simd`, `help`.

## How this area fits the system

xtask is the standard Cargo pattern for workspace automation without shell scripts. CI workflows call `cargo xtask <command>` rather than invoking Python scripts or shell one-liners directly. The task runner also coordinates with Python scripts in `scripts/` (e.g. `generate_rust_fixtures.py`) for fixture generation. It has no `[lib]` target and is never imported by other crates.

## Common edit paths

- `src/main.rs` — adding a new top-level command verb or fixture subcommand.
- `src/cmd/bench.rs` — changing budget thresholds or baseline semantics.
- `src/cmd/matrix_sync.rs` or `src/cmd/guard_sync.rs` — updating CI parity checks when workflow files change.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
