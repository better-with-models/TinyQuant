# AGENTS.md — Guide for AI Agents Working in `experiments/rust-python-performance-comparison`

This directory is the canonical manual benchmark for cross-surface TinyQuant
runtime comparison. It measures five runnable surfaces across two suites:

- `py_reference`
- `py_shim`
- `py_direct`
- `rust_cpu`
- `rust_wgpu`

Suites:

- `codec`
- `search`

The benchmark is manual only. It is not part of CI.

## What this area contains

- primary responsibility: cross-surface runtime comparison with one Python
  orchestrator and one standalone Rust helper
- main entrypoints:
  - `run_benchmark.py`
  - `bench_python_reference.py` compatibility wrapper
  - `rust_runner/` standalone release-mode helper
  - `REPORT.md` historical context only
- common changes:
  - adding benchmark cases or corpora
  - refining summary/output formatting
  - extending the Rust helper without moving it into the main workspace

## Layout

```text
rust-python-performance-comparison/
├── AGENTS.md
├── README.md
├── CLAUDE.md
├── REPORT.md
├── bench_python_reference.py
├── benchmark_cases.py
├── python_surfaces.py
├── run_benchmark.py
├── rust_runner/
│   ├── Cargo.toml
│   ├── README.md
│   └── src/main.rs
└── (results land in experiments/results/<timestamp>/ — created at runtime only; do not commit)
```

## Common workflows

### Run the canonical benchmark

```bash
python experiments/rust-python-performance-comparison/run_benchmark.py
```

Run from the repo root so relative paths for the real corpus, reference
package, and Rust helper all resolve correctly.

### Run the historical wrapper

```bash
python experiments/rust-python-performance-comparison/bench_python_reference.py
```

This now forwards into `run_benchmark.py` with the reference-only codec case.

## Invariants — Do Not Violate

- keep this directory focused on manual benchmark work only
- do not rewrite `REPORT.md` into the new canonical report; preserve it as
  historical context
- do not modify `tests/reference/tinyquant_py_reference` from here
- keep `rust_runner/` standalone and outside the main Rust workspace
- treat `experiments/results/` as runtime-only output and do not commit it
- do not silently substitute a fake real corpus if
  `experiments/quantization-benchmark/data/embeddings.npy` is missing
- update this file and the local `README.md` when structure, commands, or safe
  editing rules change

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
- [Local README.md](README.md)
