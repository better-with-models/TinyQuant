# AGENTS.md — Guide for AI Agents Working in `rust`

The `rust/` directory holds the Rust port of TinyQuant. Its crates are the
authoritative implementation of the codec, I/O, SIMD kernels, bruteforce
search, CLI, pgvector adapter, Python bindings, benches, fuzz harness, and
`xtask` automation. Python in `src/tinyquant_cpu/` remains a thin reference
and will be demoted over the Phase 23–25 release chain (see
[`docs/plans/rust/`](../docs/plans/rust/)).

## What this area contains

- primary responsibility: ship the byte-parity Rust implementation of the
  TinyQuant codec, storage formats, and search backends.
- main entrypoints:
  - `crates/tinyquant-core/` — codec, codebooks, rotation cache, SIMD kernels.
  - `crates/tinyquant-io/` — codec file format, zero-copy mmap, vector I/O.
  - `crates/tinyquant-bruteforce/` — exhaustive similarity search.
  - `crates/tinyquant-cli/` and `crates/tinyquant-py/` — user-facing surfaces.
  - `xtask/` — developer automation (fixture refresh, release helpers).
- common changes: SIMD kernel updates, codec file-format extensions,
  new backends, fixture regeneration, and crate-level release prep.

## Layout

```text
rust/
├── .cargo/
├── crates/
│   ├── tinyquant-bench/
│   ├── tinyquant-bruteforce/
│   ├── tinyquant-cli/
│   ├── tinyquant-core/
│   ├── tinyquant-fuzz/
│   ├── tinyquant-io/
│   ├── tinyquant-pgvector/
│   ├── tinyquant-py/
│   └── tinyquant-sys/
├── xtask/
├── Cargo.lock
├── Cargo.toml
├── README.md
└── rust-toolchain.toml
```

## Common workflows

### Update existing behavior

1. Read the local crate `README.md` and the files you will touch before editing.
2. Keep byte-parity with Python fixtures unless a plan in
   [`docs/plans/rust/`](../docs/plans/rust/) explicitly revises the contract.
3. Run the narrowest `cargo test -p <crate>` gate first, then `cargo test
   --workspace`, then the Python parity tests under `tests/`.

### Add a new crate or module

1. Confirm the change belongs in an existing crate before creating a new one.
2. Register new crates in the workspace `Cargo.toml` and update
   [`docs/design/rust/README.md`](../docs/design/rust/README.md).
3. Add the crate to `xtask` helpers and CI workflows if it needs its own
   gate or publish step.

## Invariants — Do Not Violate

- byte-parity with Python-generated fixtures is a release gate; refresh
  fixtures via `cargo xtask fixtures refresh-codec` instead of hand-editing.
- Rust is the source of truth for codec behavior; Python matches the Rust
  implementation, not the reverse (Phase 23+).
- every `.rs` file starts with a `//!` module docstring; public items use
  `///` doc comments per `$rust-docstrings`.

## See Also

- [Root AGENTS.md](../AGENTS.md)
- [Rust design index](../docs/design/rust/README.md)
- [Rust rollout plans](../docs/plans/rust/)
