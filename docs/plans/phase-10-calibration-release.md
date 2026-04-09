---
title: "Phase 10: Calibration & Release"
tags:
  - plans
  - phase-10
  - calibration
  - release
date-created: 2026-04-08
status: complete
category: planning
---

# Phase 10: Calibration & Release

> [!info] Goal
> Implement calibration tests, finalize documentation, and prepare the
> `v0.1.0` release. After this phase, TinyQuant is publishable to TestPyPI.

## Prerequisites

- [[plans/phase-09-pgvector-adapter|Phase 9]] complete (all code implemented)

## Deliverables

| File | What | Spec |
|------|------|------|
| `tests/calibration/test_score_fidelity.py` | Pearson rho + rank preservation | [[qa/validation-plan/README\|Validation Plan VAL-01]] |
| `tests/calibration/test_compression_ratio.py` | Storage ratio verification | [[qa/validation-plan/README\|Validation Plan VAL-02]] |
| `tests/calibration/test_determinism.py` | Cross-call determinism | [[qa/validation-plan/README\|Validation Plan VAL-03]] |
| `tests/calibration/test_research_alignment.py` | Research baseline checks | [[qa/validation-plan/README\|Validation Plan VAL-04]] |
| `tests/e2e/test_full_pipeline.py` | Add E2E-05 (score fidelity) | [[qa/e2e-tests/README\|E2E Tests]] |
| `CHANGELOG.md` | Populate with v0.1.0 entries | [[CD-plan/versioning-and-changelog\|Versioning]] |
| `README.md` | Update with installation, quick start, API examples | — |

## Steps

### 1. Calibration test data

1. Create `tests/calibration/conftest.py` with fixtures:
   - `gold_corpus_10k`: 10,000 synthetic FP32 vectors at dim 768 (fixed seed)
   - `query_vectors_100`: 100 query vectors from same distribution
   - `calibrated_config`: 4-bit config with seed 42
   - `trained_codebook_10k`: codebook trained from gold corpus

### 2. Score fidelity tests

1. Write `test_pearson_rho_4bit_with_residuals` — assert rho >= 0.995
2. Write `test_pearson_rho_4bit_without_residuals` — assert rho >= 0.98
3. Write `test_pearson_rho_2bit_with_residuals` — assert rho >= 0.95
4. Write `test_top10_overlap_4bit` — assert overlap >= 80%
5. Write `test_residual_improves_rho` — with > without
6. Write `test_fidelity_stable_across_corpus_sizes` — 1K vs 10K within 0.005

### 3. Compression ratio tests

1. Write `test_4bit_compression_ratio` — assert >= 7x
2. Write `test_4bit_with_residuals_compression_ratio` — assert >= 5x
3. Write `test_no_normalization_overhead` — verify no per-block metadata

### 4. Determinism tests

1. Write `test_compress_deterministic_across_calls` — byte-identical
2. Write `test_full_pipeline_deterministic` — compress→serialize→deserialize→decompress identical

### 5. Research alignment tests

1. Write `test_random_preconditioning_eliminates_normalization` — no per-block scales
2. Write `test_two_stage_preserves_inner_product` — residual rho > stage-1-only rho
3. Write `test_4bit_is_practical_default` — 4-bit meets fidelity targets

### 6. E2E-05: Score fidelity scenario

1. Add the deferred E2E-05 scenario to `test_full_pipeline.py`

### 7. Documentation polish

1. Update `README.md` with:
   - Project description and value proposition
   - Installation instructions (`pip install tinyquant`)
   - Quick start example (compress → store → decompress → search in <10 lines)
   - Link to API docs and design docs
2. Populate `CHANGELOG.md` with `[0.1.0]` entries covering all implemented features
3. Verify `pyproject.toml` metadata is complete (description, license, classifiers, URLs)

### 8. Pre-release verification

1. Run full CI suite locally: `ruff check . && mypy . && pytest`
2. Build: `python -m build && twine check dist/*`
3. Verify `import tinyquant; print(tinyquant.__version__)` returns `0.1.0`
4. Test install from local wheel: `pip install dist/tinyquant-0.1.0-py3-none-any.whl`

### 9. Tag and release

1. Commit all changes
2. Tag: `git tag v0.1.0`
3. Push: `git push origin main --tags`
4. Release workflow publishes to TestPyPI
5. Verify TestPyPI installation works
6. Approve PyPI publication (manual gate)

## Verification

```bash
ruff check . && ruff format --check .
mypy --strict .
pytest --cov=tinyquant --cov-fail-under=90
pytest tests/calibration/ -v  # calibration suite
python -m build && twine check dist/*
```

## Estimated scope

- ~4 test files created, 1 modified
- ~2 documentation files updated
- ~15 calibration tests + 1 E2E test
- ~400 lines of tests + ~100 lines of documentation

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-09-pgvector-adapter|Phase 9]]
- [[qa/validation-plan/README|Validation Plan]]
- [[CD-plan/README|CD Plan]]
