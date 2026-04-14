# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-py/src`

This directory contains the PyO3 bindings that produce the `tinyquant_rs._core` Python extension module. The extension mirrors the `tinyquant_cpu` Python API so that Python consumers can import either package interchangeably. `lib.rs` registers the three sub-modules and the top-level module; `backend.rs`, `codec.rs`, and `corpus.rs` wrap the corresponding Rust types as PyO3 classes; `errors.rs` maps Rust errors to Python exceptions; `numpy_bridge.rs` handles zero-copy conversion between NumPy arrays and Rust slices. The fat-wheel packaging workflow (`python-fatwheel.yml`) and the `tinyquant_cpu` dev shim both depend on the API surface defined here.

## What this area contains

- primary responsibility: PyO3 extension module `tinyquant_rs._core` — codec, corpus, and backend Python classes with NumPy interop
- main entrypoints: `lib.rs` (module registration, `VERSION` constant), `codec.rs` (codec PyO3 classes), `corpus.rs` (corpus PyO3 classes), `backend.rs` (search backend bindings)
- common changes: adding or renaming Python-visible methods, adjusting NumPy dtype handling in `numpy_bridge.rs`, mapping new Rust error variants in `errors.rs`

## Layout

```text
src/
├── backend.rs
├── codec.rs
├── corpus.rs
├── errors.rs
├── lib.rs
├── numpy_bridge.rs
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
