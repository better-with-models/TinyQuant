# rust_runner

Standalone Rust helper for `experiments/rust-python-performance-comparison`.

Purpose:

- run `rust_cpu`
- run `rust_wgpu`
- emit flat JSON rows that match the Python row schema

This crate is deliberately outside the main Rust workspace. It is built only by
the experiment orchestrator:

```bash
cargo +1.87.0 run --release \
  --manifest-path experiments/rust-python-performance-comparison/rust_runner/Cargo.toml \
  -- \
  --spec <spec.json> \
  --out <rust_results.json>
```

Notes:

- `target/` here is runtime-only and should not be committed.
- `rust_wgpu` is skip-aware. If no adapter is available, the helper emits
  skipped rows instead of crashing.
- The helper reads prepared `.npy` corpora written by `run_benchmark.py`; it
  does not generate or mutate repo fixtures.
