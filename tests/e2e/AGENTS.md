# AGENTS.md — Guide for AI Agents Working in `tests/e2e`

This directory holds end-to-end pipeline tests that drive the full train → compress → decompress → search flow using the Python API. These tests are the highest-level correctness gate: they assert that the entire stack (codec + corpus + backend) produces coherent results. CI runs these on every push. Changes here happen when the end-to-end API contract changes or new pipeline variants are added.

## What this area contains

- primary responsibility: `test_full_pipeline.py` — exercises the complete TinyQuant pipeline from raw FP32 vectors through codec training, corpus ingest, and brute-force search, asserting recall and MSE within acceptable bounds
- main entrypoints: `test_full_pipeline.py`
- common changes: adjusting quality thresholds, adding new bit-width or dimension variants to the matrix, updating assertions when the pipeline API changes

## Layout

```text
e2e/
├── __init__.py
├── README.md
└── test_full_pipeline.py
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
