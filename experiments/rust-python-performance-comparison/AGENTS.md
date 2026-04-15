# AGENTS.md — Guide for AI Agents Working in `experiments/rust-python-performance-comparison`

This directory contains the throughput comparison experiment that measured the
Python reference implementation (`tinyquant_py_reference`) against the Rust
library (`tinyquant-core`) on a 335×1536 embedding corpus. The experiment
identified that Rust kernel operations (quantize, dequantize, cosine) are
5–195× faster than Python, while the Rust end-to-end batch path is
~2,800× slower due to a per-vector SVD recomputation error in
`compress_batch_parallel`. These scripts are run manually for research; they
are not part of CI.

## What this area contains

- primary responsibility: standalone throughput comparison between the Python
  reference codec and the Rust codec, including root-cause analysis of the
  end-to-end performance gap
- main entrypoints: `bench_python_reference.py` (Python throughput benchmark;
  use `tests/reference` on `PYTHONPATH`), `REPORT.md` (full findings)
- common changes: re-running `bench_python_reference.py` with updated corpus or
  more repetitions; adding a complementary `bench_rust_kernels.py` once the
  rotation-cache fix lands

## Layout

```text
rust-python-performance-comparison/
├── bench_python_reference.py
├── REPORT.md
├── AGENTS.md
└── CLAUDE.md
```

## Common workflows

### Re-run the Python benchmark

```bash
cd /path/to/TinyQuant
python experiments/rust-python-performance-comparison/bench_python_reference.py
```

The script uses `sys.path.insert(0, "tests/reference")` — run from the repo
root so the relative path resolves.

### Add a Rust-side benchmark

Once the `fix/rotation-cache-compress-path` fix lands, add
`bench_rust_kernels.py` that exercises `tinyquant-core` via the PyO3 bindings
and update REPORT.md with the post-fix numbers.

## Invariants — Do Not Violate

- keep this directory focused on its stated responsibility
- do not modify `tinyquant_py_reference` from this directory — the reference
  implementation is read-only from the experiment's perspective
- do not invent APIs, workflows, or invariants that the code does not support
- update this file when structure or safe-editing rules change

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
- [Fix Plan](../../docs/plans/rust/rotation-cache-compress-path.md)
