# AGENTS.md — Guide for AI Agents Working in `experiments`

This directory holds standalone research scripts for TinyQuant. Nothing here is part of the main test suite or CI pipeline — scripts are run manually by researchers to explore quantization trade-offs. The only current sub-experiment is `quantization-benchmark/`, which benchmarks FP32/FP16/PQ/TinyQuant compression strategies. Changes here most often involve adding new benchmark scripts or extending existing ones; do not add CI dependencies on this directory.

## What this area contains

- primary responsibility: standalone research scripts and experiments, run manually outside CI — currently contains the `quantization-benchmark/` compression comparison experiment
- main entrypoints: `quantization-benchmark/` (the active experiment subdirectory)
- common changes: adding new experiment subdirectories, extending `quantization-benchmark/` scripts with new codec configurations or metrics

## Layout

```text
experiments/
├── quantization-benchmark/
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
- [Root AGENTS.md](../AGENTS.md)
