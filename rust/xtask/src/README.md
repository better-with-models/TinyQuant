# rust/xtask/src

This directory contains the source for the `cargo xtask` task runner binary. `main.rs` is the single entry point: it parses `std::env::args()`, dispatches to the appropriate inline function or `cmd/` subcommand module, and calls `process::exit` on failure. Fixture regeneration helpers and the CI parity check are implemented directly in `main.rs`; self-contained command modules with their own state live under `cmd/`.

## What lives here

- `main.rs` — entry point; dispatches `fmt`, `lint`, `test`, `fixtures`, `docs`, `bench`, `bench-budget`, `check-matrix-sync`, `check-publish-guards`, `simd`, and `help`. Also contains all `fixtures refresh-*` helper functions and `docs_check_ci_parity`.
- `cmd/` — modules for subcommands with non-trivial internal state (see [`cmd/README.md`](cmd/README.md)).

## How this area fits the system

`main.rs` is the authoritative list of supported task verbs. Adding a new top-level verb requires a `match` arm here and, for non-trivial logic, a corresponding module in `cmd/`. The `repo_root()` helper computes the TinyQuant repo root from the xtask working directory (`rust/` → parent); all paths passed to child processes are relative to that root. The binary is built by Cargo as a workspace-external package.

## Common edit paths

- `main.rs` — adding a fixture subcommand, changing an existing script invocation, or wiring in a new `cmd/` module.
- `cmd/` — substantive command implementations.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
