# experiments

Standalone benchmark and analysis scripts that live outside the Rust workspace.
These are exploratory tools used to evaluate quantization strategies and
visualize results before or alongside the formal Criterion benchmarks. They
have no impact on the Rust build and are not imported by any crate.

## What lives here

| Path | Role |
| --- | --- |
| `quantization-benchmark/` | Python benchmark suite comparing TinyQuant codecs against FP32/FP16/uint8/PQ baselines |
| `rust-python-performance-comparison/` | Manual cross-surface runtime benchmark for Python reference, Python shim, direct extension, Rust CPU, and Rust wgpu |
| `results/` | Runtime-only benchmark output (timestamped run directories); not committed |

## How this area fits the system

Scripts in `experiments/` call into local Python and Rust TinyQuant surfaces
and operate on data files generated or staged locally. Runtime outputs under
experiment-specific `results/` directories are not committed.

## Common edit paths

- Changing benchmark scenarios or codec configurations: `quantization-benchmark/run_benchmark.py`
- Adjusting chart output: `quantization-benchmark/generate_plots.py`
- Regenerating synthetic embedding fixtures: `quantization-benchmark/generate_embeddings.py`
- Running cross-surface runtime comparison: `rust-python-performance-comparison/run_benchmark.py`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
