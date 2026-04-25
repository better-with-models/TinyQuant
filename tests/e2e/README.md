# tests/e2e

This directory contains end-to-end pipeline tests for TinyQuant. There is one test
file, `test_full_pipeline.py`, which exercises the complete path from compression
through storage, serialization, decompression, and similarity search. Unlike unit
tests, these tests assemble all three layers — codec, corpus, and backend — together
in realistic scenarios. They are kept separate from integration tests because they
verify user-visible pipeline behaviour under named scenarios (E2E-01 through
E2E-10), not just the handoff contracts between adjacent layers.

## What lives here

- `test_full_pipeline.py` — one test class, `TestFullPipeline`, containing:
  - E2E-01: 200 gold vectors at dim 128 survive compress → store → decompress →
    ingest → search; top result is the query vector itself with score > 0.8.
  - E2E-02: `PASSTHROUGH` policy stores and retrieves bit-identical FP32 vectors.
  - E2E-03: `FP16` policy stores and retrieves vectors within FP16 tolerance.
  - E2E-04: Wrong-dimension vector rejected at corpus insert with
    `DimensionMismatchError`.
  - E2E-05: 1000 vectors at dim 768 with 4-bit + residuals meets score-fidelity
    baseline (Pearson rho >= 0.995).
  - E2E-06: Insert → `to_bytes` → `from_bytes` → Codec decompress produces the
    same output as `Corpus.decompress`.
  - E2E-08: Empty corpus operations — `decompress_all` returns `{}`, backend
    accepts empty ingest and returns no search results.
  - E2E-09: Residual correction produces equal or lower mean MSE than no-residual
    compression.
  - E2E-10: Identical inputs across two corpus instances produce bitwise-identical
    decompressed vectors and identical search rankings.

  Local fixtures define the 200-vector gold corpus at dim 128 (`gold_corpus`,
  `gold_codebook`, `gold_config`) and the 1000-vector fidelity set at dim 768
  (`fidelity_vectors`, `fidelity_codebook`, `fidelity_config`). Shared fixtures
  from `tests/conftest.py` (`config_4bit`, `trained_codebook`, `sample_vector`,
  `sample_vectors`, `corpus_compress`) are also used.

- `__init__.py` — empty package marker.

## How this area fits the system

E2E tests pull in all three bounded contexts: `codec`, `corpus`, and `backend`.
They are the closest analogue to how the library is used by external callers. A
failure here that does not correspond to a failure in unit or integration tests
indicates a composition bug — something that only manifests when the layers are
wired together.

## Common edit paths

- `test_full_pipeline.py` — add a new numbered scenario (E2E-11, etc.) when a
  new user-visible pipeline feature is added.
- Update E2E-05 and E2E-10 fidelity thresholds if the research baseline changes.
- Update local fixtures (`gold_config`, `fidelity_config`) when the canonical
  test dimension or bit width changes.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
