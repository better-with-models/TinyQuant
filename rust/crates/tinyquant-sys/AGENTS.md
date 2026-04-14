# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-sys`

This crate is the C ABI façade for TinyQuant. It exposes a stable `extern "C"` surface for non-Rust consumers (Python via ctypes/cffi, C, or other FFI clients). `build.rs` regenerates `include/tinyquant.h` via `cbindgen` on every build; CI fails on any header drift. Non-Rust embedding, the Python `tinyquant-py` PyO3 crate (which uses a separate binding path), and any future C integration depend on the handle types and error conventions established here. Changes most often happen when adding new codec or corpus ABI entry points, bumping the crate version (which requires updating `TINYQUANT_H_VERSION`), or adjusting the cbindgen configuration.

## What this area contains

- primary responsibility: `extern "C"` entry points for codec and corpus operations, opaque handle types, caller-owned error structs, and the cbindgen-generated `include/tinyquant.h` header
- main entrypoints: `src/lib.rs` (module declarations, public re-exports, `TINYQUANT_H_VERSION` compile-time parity check), `src/codec_abi.rs`, `src/corpus_abi.rs`
- common changes: adding `extern "C"` functions, updating `TINYQUANT_H_VERSION` when bumping crate version, adjusting `cbindgen.toml`, regenerating the C header

## Layout

```text
tinyquant-sys/
├── include/
├── src/
├── build.rs
├── Cargo.toml
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
- [Root AGENTS.md](../../../AGENTS.md)
