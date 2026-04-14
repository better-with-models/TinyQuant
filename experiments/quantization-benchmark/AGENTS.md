# AGENTS.md — Guide for AI Agents Working in `experiments/quantization-benchmark`

This directory contains the FP32/FP16/PQ/TinyQuant compression benchmark. It compares storage size, compression ratio, fidelity (Pearson ρ, MSE, top-k recall), and compress/decompress throughput across FP32, FP16, uint8 scalar quantization, Product Quantization simulation, and TinyQuant 8-bit/4-bit/2-bit configurations (with and without FP16 residuals). These scripts are run manually for research; they are not part of CI. Changes here most often involve adding new codec configurations to `run_benchmark.py`, extending plots in `generate_plots.py`, or regenerating the embedding fixtures via `generate_embeddings.py`.

## What this area contains

- primary responsibility: standalone benchmark comparing FP32, FP16, uint8, PQ, and TinyQuant variants on storage, fidelity, and throughput metrics — `generate_embeddings.py` (creates test corpus), `run_benchmark.py` (runs all codec comparisons, writes JSON results to `results/`), `generate_plots.py` (reads results and renders charts)
- main entrypoints: `run_benchmark.py` (primary script; read its module docstring for the full list of compared codecs), `generate_embeddings.py` (run first if `data/` is empty)
- common changes: adding new TinyQuant configurations to `run_benchmark.py`, adjusting metric thresholds, updating `generate_plots.py` to visualize new result columns

## Layout

```text
quantization-benchmark/
├── data/
├── results/
├── generate_embeddings.py
├── generate_plots.py
├── README.md
├── REPORT.md
└── run_benchmark.py
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
