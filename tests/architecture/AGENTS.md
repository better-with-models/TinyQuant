# AGENTS.md — Guide for AI Agents Working in `tests/architecture`

This directory contains static architecture tests that enforce import-direction rules across the Python packages. It has no runtime dependency on the codec or corpus implementation — it imports only module metadata. CI runs these tests to catch accidental layering violations before they reach production. Changes here happen when package boundaries are restructured or new packages are added that must respect the existing dependency direction.

## What this area contains

- primary responsibility: `test_dependency_direction.py` — asserts that lower-level packages do not import from higher-level ones (e.g., `tinyquant_cpu.codec` must not import from `tinyquant_cpu.corpus`)
- main entrypoints: `test_dependency_direction.py`
- common changes: adding new packages to the dependency matrix when the project structure grows

## Layout

```text
architecture/
├── __init__.py
├── README.md
└── test_dependency_direction.py
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
