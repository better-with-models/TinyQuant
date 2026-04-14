# tests/reference/tinyquant_py_reference/tools

This directory contains internal tooling scripts for the Python reference
implementation. Currently there is one script, `dump_serialization.py`, which
generates canonical binary fixtures for the Rust `tinyquant-io` crate's byte-parity
tests. The script is run as a standalone program rather than as a pytest test,
and its output is committed to `rust/crates/tinyquant-io/tests/fixtures/`.

## What lives here

- `dump_serialization.py` — standalone fixture generator. Defines a canonical
  table of 10 test cases (`case_01` through `case_10`) covering combinations of
  bit_width (2, 4, 8), dimension (1, 15, 16, 17, 768, 1536), and residual
  (True/False). For each case it:
  1. Generates deterministic indices and residual bytes using `numpy.random.default_rng(seed)`.
  2. Computes the canonical `config_hash` (SHA-256 of the `CodecConfig` repr string).
  3. Constructs a `CompressedVector` and serializes it to bytes via `to_bytes()`.
  4. Writes `indices.u8.bin`, `residual.u8.bin` (if residual), `config_hash.txt`,
     and `expected.bin` into a per-case subdirectory under
     `rust/crates/tinyquant-io/tests/fixtures/<case_id>/`.
  5. Appends a manifest entry with SHA-256 and byte length to `manifest.json`.

  Can be run directly:

  ```bash
  python tests/reference/tinyquant_py_reference/tools/dump_serialization.py
  ```

  Or via the dispatcher:

  ```bash
  python scripts/generate_rust_fixtures.py serialization
  ```

- `__init__.py` — empty package marker.

## How this area fits the system

The fixtures produced by `dump_serialization.py` are the source of truth for the
Rust serialization format. The Rust integration tests in
`rust/crates/tinyquant-io/tests/` load `expected.bin` and compare it against the
Rust `to_bytes` output to ensure byte-level parity between the Python reference
and the Rust implementation. Any change to `CompressedVector.to_bytes()` or the
header format in the Python reference must be followed by re-running this script
and committing the updated fixtures.

## Common edit paths

- `dump_serialization.py` — add new cases to `_CASES` when the test matrix needs
  to cover a new dimension, bit width, or residual combination.
- `dump_serialization.py` — update `_config_hash_for` if the canonical string
  format of `CodecConfig` changes.
- Run the script and commit updated fixtures in `rust/crates/tinyquant-io/tests/fixtures/`
  whenever the `CompressedVector` binary format is modified.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
