---
title: "Phase 7: Architecture & E2E Tests"
tags:
  - plans
  - phase-7
  - testing
  - architecture
  - e2e
date-created: 2026-04-08
status: complete
category: planning
---

# Phase 7: Architecture & E2E Tests

> [!info] Goal
> Add architecture enforcement tests (dependency direction, import rules)
> and full end-to-end pipeline tests. No new production code — this phase
> validates the integration of everything built so far.

## Prerequisites

- [[plans/phase-06-serialization|Phase 6]] complete (serialization works)

## Deliverables

| File | What | Spec |
|------|------|------|
| `tests/architecture/test_dependency_direction.py` | ~5 architecture tests | [[qa/verification-plan/README\|Verification Plan V-01]] |
| `tests/integration/test_codec_corpus.py` | ~4 integration tests | [[qa/integration-plan/README\|Integration Plan IB-01]] |
| `tests/integration/test_corpus_backend.py` | ~4 integration tests | [[qa/integration-plan/README\|Integration Plan IB-02]] |
| `tests/e2e/test_full_pipeline.py` | ~8 E2E scenarios | [[qa/e2e-tests/README\|E2E Tests]] |
| `tests/conftest.py` | Add higher-level fixtures | — |

## Steps

### 1. Architecture tests

1. Write `test_codec_does_not_import_corpus` — import `tinyquant.codec`, assert no corpus modules in `sys.modules`
2. Write `test_codec_does_not_import_backend` — same pattern
3. Write `test_corpus_does_not_import_backend` — same pattern
4. Write `test_no_package_cycles` — verify acyclic import graph
5. Write `test_init_exports_match_all` — verify `__all__` is consistent

### 2. Codec → Corpus integration tests

1. Write `test_codec_output_accepted_by_corpus` — compress a vector, insert into corpus
2. Write `test_codec_config_hash_matches_corpus` — hash propagation
3. Write `test_corpus_decompress_uses_same_codec` — compare direct vs corpus decompression
4. Write `test_batch_compress_integrates_with_batch_insert`

### 3. Corpus → Backend integration tests

1. Write `test_decompress_all_produces_valid_backend_input` — feed decompress_all output to BruteForceBackend.ingest
2. Write `test_decompressed_vectors_have_correct_dimension`
3. Write `test_backend_search_after_corpus_decompress` — full read path
4. Write `test_vector_ids_preserved_through_handoff`

### 4. E2E pipeline tests

1. **E2E-01:** Gold corpus compress → store → decompress → search
2. **E2E-02:** Passthrough preserves exact vectors
3. **E2E-03:** FP16 preserves approximate vectors
4. **E2E-04:** Cross-config insertion rejected
5. **E2E-06:** Serialization round trip through storage
6. **E2E-08:** Empty corpus operations
7. **E2E-09:** Residual vs no-residual quality comparison
8. **E2E-10:** Determinism across function calls

(E2E-05 score fidelity and E2E-07 pgvector are deferred to Phases 10 and 9)

### 5. Update fixtures

1. Add `sample_corpus` fixture (pre-populated corpus with 100 vectors)
2. Add `brute_force_backend` fixture
3. Add `gold_corpus_vectors` fixture (1000 synthetic vectors at dim 768)

## Verification

```bash
ruff check . && ruff format --check .
mypy --strict .
pytest tests/architecture/ -v
pytest tests/integration/ -v
pytest tests/e2e/ -v
pytest --cov=tinyquant --cov-fail-under=90  # overall
```

## Estimated scope

- 0 source files modified
- ~5 test files created, 1 modified
- ~21 tests
- ~600 lines of tests

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-06-serialization|Phase 6]]
- [[plans/phase-08-ci-cd-workflows|Phase 8: CI/CD Workflows]]
