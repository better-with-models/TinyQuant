# rust/crates/tinyquant-cli/src

This directory contains the source for the `tinyquant` CLI binary. `main.rs` owns the `clap` derive layout (the single source of truth for the subcommand tree), top-level dispatch, exit-code mapping, and the 8 MiB worker-thread workaround for Windows stack limits. Per-subcommand logic lives in the `commands/` sub-module, and format-aware I/O helpers live in `io.rs`. Progress bar wrappers are in `progress.rs`. The CLI surface mirrors Phase 22.C of `docs/plans/rust/phase-22-pyo3-cabi-release.md`.

## What lives here

- `main.rs` — binary entry point; defines `Cli`, `Command`, `CodecCmd`, `CorpusCmd`, `VectorFormat`, `SearchOutputFormat`, `IngestPolicy`, and the `Shell` enum; wires them through `dispatch()` to `commands/`.
- `commands/` — one module per top-level subcommand (`info`, `codec`, `corpus`, `verify`) plus shell-completion and man-page generators; also defines `CliErrorKind` and its exit-code mapping.
- `io.rs` — format-aware vector reader/writer for `f32`, `.npy`, CSV, and JSON-lines; consumed by the compress/decompress command paths.
- `progress.rs` — thin `indicatif` wrappers; bars write to stderr so `--format json > out.json` stays clean.

Exit codes: `0` success, `2` bad args, `3` I/O error, `4` verify failure, `70` other.

## How this area fits the system

This is the only binary crate in the workspace. It depends on `tinyquant-core` and `tinyquant-io` for codec and corpus operations, and optionally uses `jemallocator` (non-MSVC hosts). The CLI is exercised by the integration tests in `tests/cli_smoke.rs`. Output formatting (JSON, table, CSV) and input encoding are handled entirely inside this directory; no other crate reads these types.

## Common edit paths

- `commands/codec.rs` or `commands/corpus.rs` — adding flags or modifying subcommand logic.
- `io.rs` — supporting a new vector format.
- `main.rs` — only when a new top-level subcommand or global flag is introduced.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
