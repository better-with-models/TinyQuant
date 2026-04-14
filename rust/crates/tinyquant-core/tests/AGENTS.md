# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/tests`

This directory contains the integration test suite for `tinyquant-core`. Tests
exercise the public API end-to-end: `Codebook` construction and Python fixture
parity at all supported bit widths, `Codec` compress / decompress round-trips
and codec fixture parity, `Corpus` aggregate lifecycle (insert, decompress,
remove, events, batch atomicity, three-policy round-trips), SIMD kernel parity
across scalar / AVX2 / NEON / portable / every-dispatch paths, rotation matrix
fixture parity, serialization fixture parity, and a batch determinism / parallel
correctness suite. The `common/` subdirectory provides shared helpers. Binary
fixtures live in `fixtures/`.

## What this area contains

- primary responsibility: integration tests for the `tinyquant-core` public API — codebook, codec service, corpus aggregate, SIMD parity, serialization fixture parity, and batch / parallel correctness
- main entrypoints: `codec_service.rs`, `codebook.rs`, `corpus_aggregate.rs`, `simd_parity_all_dispatch.rs`, `codec_fixture_parity.rs`
- common changes: adding a new parity test when a fixture file is regenerated, extending corpus event or policy coverage, adding a SIMD parity file for a new kernel

## Layout

```text
tests/
├── common/
├── fixtures/
├── backend_trait.rs
├── codebook.rs
├── codec_config.rs
├── codec_fixture_parity.rs
├── codec_service.rs
├── compressed_vector.rs
├── compression_policy.rs
├── corpus_aggregate.rs
├── corpus_events.rs
└── corpus_insertion_order.rs
└── …
```

## Common workflows

### Update existing behavior

1. Read the local README and the files you will touch before editing.
2. Follow the local invariants before introducing new files or abstractions.
3. Update nearby docs when the change affects layout, commands, or invariants.
4. Run the narrowest useful verification first, then the broader project gate.

### Add a new file or module

1. Confirm the new file belongs in this directory rather than a sibling.
2. Update the layout section if the structure changes in a way another agent must notice.
3. Add or refine local docs when the new file introduces a new boundary or invariant.

## Invariants — Do Not Violate

- keep this directory focused on its stated responsibility
- do not invent APIs, workflows, or invariants that the code does not support
- update this file when structure or safe-editing rules change

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
