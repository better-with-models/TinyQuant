# rust/xtask/src/cmd

This directory contains the self-contained subcommand modules for `cargo xtask`. Each file implements a focused task that would be too large or stateful to inline in `main.rs`. All modules are declared in `main.rs` under `mod cmd { pub mod ...; }` and called via their `run()` entry point.

## What lives here

- `bench.rs` — Criterion benchmark budget commands: `--capture-baseline <name>`, `--check-against <name>`, `--validate`, and `--diff <from> <to>`. A regression is detected when `new_median > baseline_median * 1.10` (10 % budget). Baselines live in `rust/baselines/*.json`.
- `guard_sync.rs` — Phase 22.D publish-guard drift check: parses `.github/workflows/rust-release.yml` and asserts the `if:` expressions on all four `publish-*` jobs are byte-identical.
- `guard_sync_python.rs` — Phase 24.3 extension: enforces the single-job publish contract on `.github/workflows/python-fatwheel.yml`.
- `matrix_sync.rs` — Phase 22.D CLI smoke matrix check: diffs the target triples listed in the plan doc against the `strategy.matrix.include` in `rust-release.yml`; order-insensitive but membership-strict.
- `simd.rs` — Phase 20 SIMD audit: asserts that the required CI job names (`simd-scalar`, `simd-avx2`, `simd-neon`, `miri-scalar`, `simd-neon-darwin`, `simd-avx2-windows`) are present in `rust-ci.yml`.

## How this area fits the system

These modules are invoked by `main.rs` and have no callers outside `xtask`. They read workflow YAML and plan docs from the repository (using `repo_root()` from `main.rs`) and exit non-zero on drift — this is the mechanism that prevents CI configuration from silently diverging from design docs (Phase 14 Lesson L3 pattern).

## Common edit paths

- `bench.rs` — adjusting the regression budget percentage or baseline schema.
- `guard_sync.rs` — if the set of `publish-*` job names in `rust-release.yml` changes.
- `matrix_sync.rs` — if the set of supported target triples changes in the plan doc or release workflow.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
