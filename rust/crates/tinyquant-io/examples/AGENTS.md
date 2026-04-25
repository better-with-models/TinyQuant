# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-io/examples`

This directory contains the developer example that generates golden corpus
fixture files for the `tinyquant-io` integration tests. `gen_corpus_fixture.rs`
writes `tests/fixtures/codec_file/golden_100.tqcv` (100 vectors, dim=64,
bw=4) and a matching raw `u8` index binary using deterministic
`ChaCha20Rng::seed_from_u64(17)`. The generated files are the ground truth for
fixture-parity integration tests in `tinyquant-io/tests/`. Run with
`cargo run -p tinyquant-io --example gen_corpus_fixture` to regenerate after
a wire format change.

## What this area contains

- primary responsibility: fixture generation for `tinyquant-io` integration tests — `gen_corpus_fixture.rs` writes a golden `.tqcv` file and a companion raw index binary
- main entrypoints: `gen_corpus_fixture.rs` (single file; uses `CodecFileWriter` and `CompressedVector`)
- common changes: updating the vector count, dimension, or bit-width constants when the fixture spec changes, regenerating after a `codec_file` header format update

## Layout

```text
examples/
├── gen_corpus_fixture.rs
└── README.md
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
