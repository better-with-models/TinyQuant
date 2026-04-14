# rust/crates/tinyquant-cli/tests

This directory contains the end-to-end smoke tests for the `tinyquant` CLI binary. The tests drive the compiled binary via `assert_cmd::Command::cargo_bin("tinyquant")` — exactly as an operator would — and assert on exit status, stdout content, and on-disk artefacts. They do not import any internal crate modules; all interaction is through the public CLI surface.

## What lives here

- `cli_smoke.rs` — three integration tests that mirror the §Step 12 contract from `docs/plans/rust/phase-22-pyo3-cabi-release.md`:
  1. `info_subcommand_prints_version` — checks the binary starts and prints version metadata.
  2. `codec_train_writes_codebook_file` — verifies argument parsing, `tinyquant-io` wiring, and codebook file output.
  3. `codec_compress_decompress_round_trip_via_cli` — full pipeline: train → compress → decompress, asserting MSE < 1e-2.

## How this area fits the system

These tests are the primary guard that the CLI binary compiles and operates end-to-end. They depend on the `assert_cmd`, `predicates`, `rand`, and `rand_chacha` crates; fixture data is generated in-test using a fixed seed for determinism. Because they shell out to the binary, they implicitly exercise `tinyquant-core` and `tinyquant-io` through the public CLI surface. `cargo test -p tinyquant-cli` runs them.

## Common edit paths

- `cli_smoke.rs` — the only test file; extend it when a new subcommand or flag is added that has an observable artefact or output.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
