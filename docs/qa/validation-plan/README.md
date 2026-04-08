---
title: Validation Plan
tags:
  - qa
  - validation
  - meta
date-created: 2026-04-08
status: active
category: qa
---

# Validation Plan

> [!info] Purpose
> **"Did we build the right thing?"** — checks that TinyQuant delivers the
> value it was designed for: meaningful storage compression with acceptable
> score fidelity, aligned with the research baselines.

## Validation vs verification

| | Verification | Validation |
|-|-------------|-----------|
| **Question** | Did we build it right? | Did we build the right thing? |
| **Checks** | Architecture, types, coverage, invariants | Fidelity, compression ratio, usability, research alignment |
| **Audience** | Engineering | Product, research, downstream consumers |
| **Timing** | Every commit | Release gates and periodic calibration |

## Validation dimensions

### VAL-01: Score fidelity

**Validates:** [[design/behavior-layer/score-fidelity|Score Fidelity Behavior]]

| Metric | Target | Method |
|--------|--------|--------|
| Pearson rho (4-bit + residuals) | >= 0.995 | Calibration suite against gold corpus |
| Pearson rho (4-bit, no residuals) | >= 0.98 | Calibration suite |
| Pearson rho (2-bit + residuals) | >= 0.95 | Calibration suite |
| Top-10 neighbor overlap (4-bit + residuals, 10K corpus) | >= 80% | Calibration suite |
| Top-10 neighbor overlap (4-bit + residuals, 50K corpus) | >= 75% | Calibration suite |

**Test file:** `tests/calibration/test_score_fidelity.py`

**Data requirements:**
- Gold corpus: 10,000+ real or realistic FP32 embeddings at dim 768
- Query set: 100+ vectors from the same distribution
- Ground truth: precomputed pairwise similarities and top-k neighbors on
  uncompressed vectors

> [!warning] Distribution sensitivity
> Fidelity numbers depend on the embedding distribution. Calibration must
> run against representative data, not random noise. Results on synthetic
> data are necessary for CI but not sufficient for release validation.

### VAL-02: Compression ratio

| Metric | Target | Method |
|--------|--------|--------|
| Storage ratio (4-bit vs FP32) | >= 7x | Compute `original_bytes / compressed_bytes` |
| Storage ratio (4-bit + residuals vs FP32) | >= 5x | Include residual overhead |
| Overhead from normalization metadata | 0 bytes per vector | Verify no per-block scales stored |

**Test file:** `tests/calibration/test_compression_ratio.py`

**Why this matters:** the research claims ~8x compression at 4-bit. If the
implementation adds overhead (metadata, padding, headers) that reduces this
significantly, the value proposition weakens.

### VAL-03: Determinism

| Metric | Target | Method |
|--------|--------|--------|
| Cross-run determinism | Byte-identical CompressedVectors | Run pipeline twice with same inputs across process boundaries |
| Cross-platform determinism | Byte-identical (stretch goal) | CI on Linux + macOS + Windows |

**Test file:** `tests/calibration/test_determinism.py`

**Why this matters:** downstream systems may store compressed vectors
durably. Non-deterministic compression would make debugging and migration
unpredictable.

### VAL-04: Research baseline alignment

| Claim from research | Validation |
|--------------------|-----------|
| Random preconditioning eliminates normalization overhead | Verify no per-block scales in CompressedVector format |
| Two-stage quantization preserves inner-product fidelity | Compare Pearson rho with and without residuals |
| 4-bit is the practical default | 4-bit fidelity meets targets on representative data |
| Compressed-domain search is too slow on CPU | Benchmark compressed-domain vs FP32 search latency |
| Score fidelity is independent of corpus size | Fidelity stable across 1K, 10K, 50K corpora |

**Test file:** `tests/calibration/test_research_alignment.py`

### VAL-05: API usability

| Check | Method |
|-------|--------|
| A new user can compress and decompress in < 10 lines of code | Code example in README and docstrings |
| Error messages are actionable (include expected vs actual) | Manual review of all custom exception messages |
| The public API uses domain vocabulary consistently | Cross-reference API names against [[domain-layer/ubiquitous-language\|Ubiquitous Language]] |
| Sphinx-generated docs are complete and navigable | Sphinx build + manual review |

### VAL-06: Compression policy correctness

**Validates:** [[design/behavior-layer/corpus-management|Corpus Management Behavior]]

| Policy | Validation |
|--------|-----------|
| `COMPRESS` | Vectors are actually smaller; decompressed scores meet fidelity targets |
| `PASSTHROUGH` | Vectors are bit-identical to originals; Pearson rho = 1.0 |
| `FP16` | Vectors are FP16 precision; error bounded by FP16 epsilon |

### VAL-07: Acceptance criteria traceability

Every BDD scenario from the [[design/behavior-layer/README|Behavior Layer]]
must trace to at least one automated test:

| Behavior spec | Automated in |
|--------------|-------------|
| [[design/behavior-layer/codec-compression\|Codec Compression]] | `tests/codec/test_codec.py` + `tests/e2e/test_full_pipeline.py` |
| [[design/behavior-layer/codec-decompression\|Codec Decompression]] | `tests/codec/test_codec.py` + `tests/e2e/test_full_pipeline.py` |
| [[design/behavior-layer/corpus-management\|Corpus Management]] | `tests/corpus/test_corpus.py` + `tests/e2e/test_full_pipeline.py` |
| [[design/behavior-layer/score-fidelity\|Score Fidelity]] | `tests/calibration/test_score_fidelity.py` |
| [[design/behavior-layer/backend-protocol\|Backend Protocol]] | `tests/backend/test_brute_force.py` + `tests/e2e/test_full_pipeline.py` |

## Validation schedule

| When | What |
|------|------|
| **Every CI run** | Synthetic-data calibration (fast, reproducible, necessary but not sufficient) |
| **Pre-release** | Full calibration against representative embedding data |
| **Post-release** | Downstream consumer feedback; score fidelity monitoring in production integrations |

## Calibration data management

| Dataset | Purpose | Storage |
|---------|---------|---------|
| Synthetic (fixed seed) | CI reproducibility | Generated at test time |
| Representative embeddings | Release validation | Stored in `tests/fixtures/` (git-lfs or downloaded) |
| Production sample | Post-release monitoring | Consumer-provided; not stored in TinyQuant repo |

> [!tip] Reproducibility
> Calibration tests using synthetic data must use fixed seeds and be fully
> deterministic. Tests using representative data should pin the dataset
> version and document its provenance.

## See also

- [[qa/README|Quality Assurance]]
- [[qa/verification-plan/README|Verification Plan]]
- [[design/behavior-layer/README|Behavior Layer]]
- [[design/behavior-layer/score-fidelity|Score Fidelity Behavior]]
- [[design/domain-layer/ubiquitous-language|Ubiquitous Language]]
