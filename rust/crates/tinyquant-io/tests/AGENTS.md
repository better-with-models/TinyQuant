# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-io/tests`

Integration and property tests for the `tinyquant-io` serialization layer.
Tests cover bit-packing exhaustion, round-trip file correctness, memory-mapped
corpus access, zero-copy views, rejection of malformed input, and byte-level
parity with the Python reference implementation.

## What this area contains

- primary responsibility: validate that the serialization layer is correct, stable, and compatible with the Python reference
- main entrypoints: `roundtrip.rs` (happy-path encode/decode), `bit_pack_exhaustive.rs` (all bit-width combinations), `python_parity.rs` (cross-impl byte comparison)
- common changes: adding new fixture dimensions, tightening rejection criteria, extending the bit-pack matrix

## Layout

```text
tests/
├── baselines/
├── fixtures/
├── bit_pack_exhaustive.rs
├── codec_file_proptest.rs
├── header_size_audit.rs
├── mmap_corpus.rs
├── python_parity.rs
├── README.md
├── rejection.rs
├── roundtrip.rs
├── smoke.rs
└── zero_copy.rs
```

## Common workflows

### Update existing behavior

1. Read the test file and the relevant source module before editing.
2. If the binary format changes, regenerate fixtures with `cargo run --example gen_corpus_fixture`.
3. Run `cargo test -p tinyquant-io` to confirm no regressions.

### Add a new test file

1. Add the file under `tests/` and register it in `Cargo.toml` `[[test]]` if needed.
2. Update the layout section above.
3. Add or update fixtures in `fixtures/` when the new test needs golden data.

## Invariants — Do Not Violate

- Fixture files in `fixtures/` are committed golden data; do not regenerate silently.
- `python_parity.rs` must stay in sync with the Python reference byte layout.
- `rejection.rs` must cover every documented invalid-header variant.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
