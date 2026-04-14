# tests/calibration

This directory contains the quality-gate calibration tests (VAL-01 through VAL-04).
Unlike the unit tests in `codec/` and `corpus/`, these tests operate on a large
synthetic corpus — 10,000 FP32 vectors at dimension 768 — to measure real-world
compression and fidelity behaviour. They are slow because they train codebooks and
run batch compression over thousands of vectors. Module-scoped fixtures in
`conftest.py` amortize the expensive setup across the tests in each file. The
parent test suite documents these tests under the `calibration` marker, used to
exclude them from fast CI runs (`pytest -m "not calibration"`).

## What lives here

- `conftest.py` — module-scoped fixtures: a 10,000-vector corpus at dim 768
  (`gold_corpus_10k`), 100 query vectors (`query_vectors_100`), a shared `Codec`
  instance, and codebooks trained under 4-bit with/without residuals and 2-bit
  with residuals.
- `test_score_fidelity.py` (VAL-01) — asserts Pearson rho thresholds for cosine
  similarity ranking: 4-bit with residuals (>= 0.995), 4-bit without residuals
  (>= 0.97), 2-bit with residuals (>= 0.95); top-10 neighbor overlap >= 80%;
  and that residual correction strictly improves rho.
- `test_compression_ratio.py` (VAL-02) — verifies that 4-bit without residuals
  achieves >= 7x payload compression, 4-bit with FP16 residuals achieves >= 1.5x,
  and that no per-block normalization metadata appears in the serialized bytes.
- `test_determinism.py` (VAL-03) — verifies that the same inputs produce
  byte-identical `CompressedVector` output across repeated calls, and that the
  full compress → serialize → deserialize → decompress pipeline is deterministic.
- `test_research_alignment.py` (VAL-04) — checks three research baseline claims:
  random preconditioning eliminates per-block normalization overhead, two-stage
  (residual) quantization improves mean Pearson rho versus stage-1 only, and
  4-bit with residuals meets the >= 0.995 fidelity and >= 1.5x compression targets
  simultaneously.
- `__init__.py` — marks the directory as a package.

## How this area fits the system

All four test files depend on fixtures from `conftest.py` in this directory.
They exercise `Codec`, `Codebook`, `CodecConfig`, and `CompressedVector` from
`tinyquant_py_reference.codec` against the 10K synthetic corpus. The results
define the numerical floor that the Rust port must also meet, so any regression
here blocks a release.

## Common edit paths

- `conftest.py` — adjust corpus size, dimension, or add new fixture variants
  (e.g., 8-bit codebooks) when extending the calibration matrix.
- `test_score_fidelity.py` — update Pearson rho thresholds if the research
  baseline changes.
- `test_compression_ratio.py` — update ratio assertions when the serialization
  format changes (e.g., a new header field).

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
