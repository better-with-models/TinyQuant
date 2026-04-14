# AGENTS.md — Guide for AI Agents Working in `tests/reference/tinyquant_py_reference/tools`

This directory holds developer utility scripts for the Python reference implementation. Tools here generate or inspect fixture data used by the Rust byte-parity tests; they are invoked by `cargo xtask fixtures refresh-serialization` or directly from the repo root. Changes here happen when the serialization format changes and the fixture generator must be updated to match.

## What this area contains

- primary responsibility: `dump_serialization.py` — generates `CompressedVector` byte-parity fixture files consumed by `tinyquant-io` Rust tests; also prints SHA-256 hashes for manual verification
- main entrypoints: `dump_serialization.py` (run via `cargo xtask fixtures refresh-serialization` or `python scripts/generate_rust_fixtures.py serialization`)
- common changes: updating the serialization layout when the `CompressedVector` binary format changes, adding new bit-width or dimension cases to the fixture set

## Layout

```text
tools/
├── __init__.py
├── dump_serialization.py
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
