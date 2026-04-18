# Cross-Surface Performance Benchmark

Manual benchmark for TinyQuant runtime comparison across the runnable Python
and Rust surfaces.

Surfaces:

- `py_reference` — frozen pure-Python oracle: `tinyquant_py_reference`
- `py_shim` — wrapped Python namespace: `tinyquant_cpu`
- `py_direct` — direct PyO3 package: `tinyquant_rs`
- `rust_cpu` — direct Rust CPU path: `tinyquant-core` + `tinyquant-bruteforce`
- `rust_wgpu` — direct Rust GPU path: `tinyquant-gpu-wgpu`

Suites:

- `codec` — batch compress and batch decompress
- `search` — top-k cosine search

Shared codec config:

- `bit_width=4`
- `seed=42`
- `dimension=1536`
- `residual_enabled=false`

The GPU codec line is limited to `residual_enabled=false` because the current
`wgpu` backend does not support residual-enabled compression.

## Prerequisites

- Run from the repo root.
- Real corpus must exist at
  `experiments/quantization-benchmark/data/embeddings.npy`.
- Python benchmark lines:
  - `py_reference` uses `tests/reference`
  - `py_shim` and `py_direct` require `tinyquant_rs` to be installed, typically
    via `maturin develop`
- Rust helper:
  - Rust `1.87.0`
  - optional GPU adapter for `rust_wgpu`

## Commands

Default full benchmark:

```bash
python experiments/rust-python-performance-comparison/run_benchmark.py
```

Example narrow runs:

```bash
python experiments/rust-python-performance-comparison/run_benchmark.py --surfaces py_reference --suites codec
python experiments/rust-python-performance-comparison/run_benchmark.py --surfaces rust_cpu,rust_wgpu --suites search
```

Legacy compatibility wrapper:

```bash
python experiments/rust-python-performance-comparison/bench_python_reference.py
```

That wrapper now forwards into the new orchestrator with the historical
reference-only codec case.

## Outputs

Each run writes to:

`experiments/results/<timestamp>/`

Files:

- `results.json`
- `results.csv`
- `SUMMARY.md`
- `rust_spec.json`
- `rust_results.json`
- normalized/generated `.npy` corpus artifacts under `corpora/`

`experiments/results/` is runtime-only and should not be committed.

## Skip Behavior

- If `tinyquant_rs` is unavailable:
  - `py_shim` skipped
  - `py_direct` skipped
  - reason: `tinyquant_rs not installed; build with maturin develop`
- If no `wgpu` adapter is available:
  - `rust_wgpu` skipped
  - reason: `no wgpu adapter available`
- If the real corpus file is missing:
  - the orchestrator hard-fails

## Historical Context

`REPORT.md` is retained as the historical one-off analysis. The canonical
manual benchmark is now `run_benchmark.py`.
