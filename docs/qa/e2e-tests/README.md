---
title: End-to-End Tests
tags:
  - qa
  - testing
  - e2e
  - meta
date-created: 2026-04-08
status: active
category: qa
---

# End-to-End Tests

> [!info] Purpose
> Full-pipeline test scenarios that exercise TinyQuant from raw FP32 vectors
> through compression, storage, decompression, and search. These validate
> that the system works as a whole, not just in isolated units.

## Test file

```text
tests/e2e/test_full_pipeline.py
```

## Relationship to behavior specs

Each E2E scenario here corresponds to one or more BDD scenarios from the
[[design/behavior-layer/README|Behavior Layer]]. The behavior specs define
the *what*; these E2E specs define the *how* at test implementation level.

## Scenarios

### E2E-01: Gold corpus compress-store-decompress-search

**Traces to:** [[design/behavior-layer/codec-compression|Codec Compression]],
[[design/behavior-layer/codec-decompression|Codec Decompression]],
[[design/behavior-layer/backend-protocol|Backend Protocol]]

```text
Given  a CodecConfig (4-bit, seed 42, dim 768, residuals enabled)
And    a Codebook trained from 5000 representative vectors
And    a Corpus with compression_policy COMPRESS
And    a BruteForceBackend

When   1000 FP32 vectors are inserted into the corpus
And    the corpus is batch-decompressed
And    decompressed vectors are ingested into the backend
And    10 query vectors are searched with top_k=10

Then   all 1000 vectors are stored in the corpus
And    all 1000 decompress to FP32 arrays of dimension 768
And    each search returns exactly 10 ranked results
And    results are ordered by descending cosine similarity
```

### E2E-02: Passthrough corpus preserves exact vectors

**Traces to:** [[design/behavior-layer/corpus-management|Corpus Management]],
[[design/behavior-layer/score-fidelity|Score Fidelity]]

```text
Given  a Corpus with compression_policy PASSTHROUGH

When   100 FP32 vectors are inserted
And    all vectors are retrieved

Then   retrieved vectors are bit-identical to the originals
And    pairwise cosine similarities are exactly preserved
```

### E2E-03: FP16 corpus preserves approximate vectors

**Traces to:** [[design/behavior-layer/corpus-management|Corpus Management]]

```text
Given  a Corpus with compression_policy FP16

When   100 FP32 vectors are inserted
And    all vectors are retrieved

Then   retrieved vectors are FP16-precision approximations
And    the maximum per-element error is bounded by FP16 precision
```

### E2E-04: Cross-config insertion rejected

**Traces to:** [[design/behavior-layer/corpus-management|Corpus Management]]

```text
Given  Corpus A with seed 42
And    Corpus B with seed 99
And    a vector compressed under Corpus B's config

When   an attempt is made to insert that vector into Corpus A

Then   the insertion is rejected with a config mismatch error
And    Corpus A remains unchanged
And    a CompressionPolicyViolationDetected event is raised
```

### E2E-05: Score fidelity meets research baseline

**Traces to:** [[design/behavior-layer/score-fidelity|Score Fidelity]]

```text
Given  a gold corpus of 10000 vectors at dimension 768
And    4-bit compression with residuals enabled
And    original pairwise cosine similarities for 1000 pairs

When   all vectors are compressed, decompressed, and similarities recomputed

Then   Pearson rho between original and decompressed scores >= 0.995
And    top-10 neighbor overlap >= 80% for 100 query vectors
```

### E2E-06: Serialization round trip through storage

**Traces to:** [[design/behavior-layer/codec-compression|Codec Compression]],
[[design/behavior-layer/codec-decompression|Codec Decompression]]

```text
Given  a CompressedVector produced by the codec

When   the vector is serialized to bytes via to_bytes()
And    deserialized back via from_bytes()
And    then decompressed

Then   the decompressed FP32 vector is identical to one decompressed
       from the original CompressedVector (before serialization)
```

### E2E-07: Backend adapter wire format (pgvector)

**Traces to:** [[design/behavior-layer/backend-protocol|Backend Protocol]]

```text
Given  a set of decompressed FP32 vectors
And    a PgvectorAdapter connected to a test database

When   vectors are ingested via the adapter
And    a search query is executed

Then   results are returned as SearchResult objects
And    vector dimensionality is preserved through the wire format
```

> [!note] Infrastructure required
> This scenario requires a PostgreSQL instance with pgvector. It runs in
> CI with a test database, not in the inner TDD loop.

### E2E-08: Empty corpus operations

```text
Given  a newly created empty corpus

When   decompress_all is called
And    a search is attempted against the backend

Then   decompress_all returns an empty dict
And    search returns an empty result list
And    no errors are raised
```

### E2E-09: Residual vs no-residual quality comparison

**Traces to:** [[design/behavior-layer/score-fidelity|Score Fidelity]]

```text
Given  identical vectors compressed at 4-bit with and without residuals

When   both sets are decompressed and Pearson rho is computed

Then   residual-enabled rho > residual-disabled rho
```

### E2E-10: Determinism across runs

```text
Given  the same CodecConfig, Codebook, and input vectors

When   the full pipeline runs twice in separate process invocations

Then   all CompressedVectors are byte-identical across runs
And    all decompressed FP32 vectors are identical across runs
```

## Execution

| Aspect | Policy |
|--------|--------|
| **Runner** | `pytest tests/e2e/` |
| **Frequency** | Every CI run |
| **Timeout** | 5 minutes max for the full suite |
| **Data** | Synthetic vectors generated from fixed seeds for reproducibility |
| **E2E-07** | Runs only when `PGVECTOR_TEST_DSN` env var is set |

## See also

- [[qa/README|Quality Assurance]]
- [[design/behavior-layer/README|Behavior Layer]]
- [[qa/unit-tests/README|Unit Tests]]
