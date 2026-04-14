# experiments

Standalone benchmark and analysis scripts that live outside the Rust workspace.
These are exploratory tools used to evaluate quantization strategies and
visualize results before or alongside the formal Criterion benchmarks. They
have no impact on the Rust build and are not imported by any crate.

## What lives here

| Path | Role |
|---|---|
| `quantization-benchmark/` | Python benchmark suite comparing TinyQuant codecs against FP32/FP16/uint8/PQ baselines |

## How this area fits the system

Scripts in `experiments/` call into the `tinyquant_cpu` Python package (the
Python bindings to the codec) and operate on data files that are generated
locally — not committed to the repo. Results and plots are written to
subdirectories inside `quantization-benchmark/` and are also not committed.

## Common edit paths

- Changing benchmark scenarios or codec configurations: `quantization-benchmark/run_benchmark.py`
- Adjusting chart output: `quantization-benchmark/generate_plots.py`
- Regenerating synthetic embedding fixtures: `quantization-benchmark/generate_embeddings.py`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
