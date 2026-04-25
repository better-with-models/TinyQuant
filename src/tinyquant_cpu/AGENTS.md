# AGENTS.md — Guide for AI Agents Working in `src/tinyquant_cpu`

This package is a developer-mode shim that re-exports the Phase 22.A `tinyquant_rs._core` extension as `tinyquant_cpu._core`. In the fat wheel produced by the packaging workflow the shim is replaced by a per-arch runtime selector; locally it allows tests and scripts to `import tinyquant_cpu` without installing the wheel. The Python test suite, calibration scripts, and parity tests all import through this package. Changes here are uncommon — they happen only when the shim contract (documented in `docs/plans/rust/phase-24-python-fat-wheel-official.md` §Shim-layer contract) or the module namespace changes.

## What this area contains

- primary responsibility: dev-mode shim re-exporting `tinyquant_rs._core` under `sys.modules["tinyquant_cpu._core"]`; sub-packages `backend/`, `codec/`, and `corpus/` mirror the production `tinyquant_cpu` namespace
- main entrypoints: `__init__.py` (the shim entry point), `_types.py` (shared type definitions used by sub-packages)
- common changes: updating the shim import path when the extension module name changes, adjusting `_types.py` when new shared types are added

## Layout

```text
tinyquant_cpu/
├── backend/
├── codec/
├── corpus/
├── tools/
├── __init__.py
├── _types.py
├── py.typed
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
- [Root AGENTS.md](../../AGENTS.md)
