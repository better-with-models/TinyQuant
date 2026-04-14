# rust/crates

This directory contains all ten library and binary crates in the TinyQuant Rust workspace. Each crate is independent and has its own `Cargo.toml`; shared version metadata is inherited from the workspace root at `rust/Cargo.toml`. Crates are listed below by name with a one-line purpose.

## What lives here

| Crate | Purpose |
|---|---|
| `tinyquant-core` | Core codec types: `CodecConfig`, `Codebook`, `RotationMatrix`, `CompressedVector`, `Corpus`, and the backend trait. |
| `tinyquant-io` | Serialization and file I/O for codec and corpus artefacts (TQCV format, codebook binary). |
| `tinyquant-bruteforce` | `BruteForceBackend` — exhaustive nearest-neighbour search over a `Corpus`. |
| `tinyquant-bench` | Criterion benchmarks for codec and corpus hot paths; consumes calibration fixtures. |
| `tinyquant-py` | PyO3 extension module (`tinyquant_rs._core`) — Python bindings built by `maturin`. |
| `tinyquant-sys` | `extern "C"` ABI facade with cbindgen-generated `tinyquant.h`; targets non-Rust consumers. |
| `tinyquant-cli` | `tinyquant` binary — subcommand tree for `info`, `codec`, `corpus`, and `verify`. |
| `tinyquant-js` | WASM / `wasm-bindgen` bindings for browser and Node.js consumers. |
| `tinyquant-pgvector` | PostgreSQL `pgvector` integration (extension or client glue). |
| `tinyquant-fuzz` | `cargo-fuzz` targets for codec and serialization fuzzing. |

## How this area fits the system

The dependency graph flows outward from `tinyquant-core`: `tinyquant-io` extends it with persistence, `tinyquant-bruteforce` provides search, and the three binding crates (`tinyquant-py`, `tinyquant-sys`, `tinyquant-js`) each wrap core and io for a specific consumer. `tinyquant-cli` depends on core, io, and bruteforce. `tinyquant-bench` and `tinyquant-fuzz` are test/CI artefacts with no production consumers.

## Common edit paths

- New codec feature: likely touches `tinyquant-core` first, then `tinyquant-io`, then binding crates.
- New CLI subcommand: `tinyquant-cli/src/commands/`.
- New Python API method: `tinyquant-py/src/codec.rs` or `corpus.rs`.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
