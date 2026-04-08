---
title: "Phase 4: Corpus Layer"
tags:
  - plans
  - phase-4
  - corpus
  - aggregate
date-created: 2026-04-08
status: pending
category: planning
---

# Phase 4: Corpus Layer

> [!info] Goal
> Implement the corpus bounded context: `CompressionPolicy` enum,
> `VectorEntry` entity, domain events, and the `Corpus` aggregate root.

## Prerequisites

- [[plans/phase-03-codec-service|Phase 3]] complete (codec compress/decompress works)

## Deliverables

| File | Class | Spec |
|------|-------|------|
| `src/tinyquant/corpus/compression_policy.py` | `CompressionPolicy` | [[classes/compression-policy\|CompressionPolicy]] |
| `src/tinyquant/corpus/vector_entry.py` | `VectorEntry` | [[classes/vector-entry\|VectorEntry]] |
| `src/tinyquant/corpus/events.py` | 4 event dataclasses | [[classes/corpus-events\|Corpus Events]] |
| `src/tinyquant/corpus/corpus.py` | `Corpus` | [[classes/corpus\|Corpus]] |
| `src/tinyquant/corpus/__init__.py` | Public re-exports | — |
| `tests/corpus/test_compression_policy.py` | ~6 tests | [[qa/unit-tests/test-compression-policy\|Test spec]] |
| `tests/corpus/test_vector_entry.py` | ~8 tests | [[qa/unit-tests/test-vector-entry\|Test spec]] |
| `tests/corpus/test_events.py` | ~10 tests | [[qa/unit-tests/test-events\|Test spec]] |
| `tests/corpus/test_corpus.py` | ~30 tests | [[qa/unit-tests/test-corpus\|Test spec]] |

## Steps (TDD order)

### 1. CompressionPolicy

1. Write `test_compress_requires_codec` — RED
2. Implement `CompressionPolicy` enum with `COMPRESS`, `PASSTHROUGH`, `FP16` — GREEN
3. Write remaining `requires_codec` and `storage_dtype` tests — RED → GREEN

### 2. VectorEntry

1. Write `test_construction_with_valid_fields` — RED
2. Implement `VectorEntry` dataclass — GREEN
3. Write delegation property tests (`config_hash`, `dimension`, `has_residual`) — RED → GREEN
4. Write equality-by-id tests — RED → GREEN

### 3. Domain events

1. Write `test_corpus_created_fields` — RED
2. Implement all 4 frozen event dataclasses — GREEN
3. Write immutability and timestamp tests for each event — RED → GREEN

### 4. Corpus aggregate — creation

1. Write `test_create_empty_corpus` — RED
2. Implement `Corpus.__init__` with frozen config, codebook, policy — GREEN
3. Write `test_create_corpus_raises_corpus_created_event` — RED → GREEN
4. Implement `pending_events()` mechanism — GREEN

### 5. Corpus aggregate — insert

1. Write `test_insert_stores_vector` — RED
2. Implement `insert()` with `COMPRESS` policy (delegates to Codec) — GREEN
3. Write `test_insert_config_hash_matches` — RED → GREEN
4. Write `test_insert_dimension_mismatch_raises` — RED → GREEN
5. Write `test_insert_duplicate_id_raises` — RED → GREEN
6. Implement `PASSTHROUGH` and `FP16` insert paths
7. Write policy-specific tests — RED → GREEN

### 6. Corpus aggregate — batch insert

1. Write `test_insert_batch_stores_all` — RED
2. Implement `insert_batch()` with atomic semantics — GREEN
3. Write `test_insert_batch_is_atomic` — RED → GREEN

### 7. Corpus aggregate — policy enforcement

1. Write `test_policy_immutable_after_first_insert` — RED → GREEN
2. Write `test_cross_config_vector_rejected` — RED → GREEN

### 8. Corpus aggregate — retrieval and decompression

1. Write `test_get_returns_correct_entry` — RED → GREEN
2. Write `test_decompress_single_returns_fp32` — RED → GREEN
3. Write `test_decompress_all_returns_all_vectors` — RED → GREEN
4. Write `test_decompress_all_raises_event` — RED → GREEN

### 9. Corpus aggregate — remove

1. Write `test_remove_decrements_count` — RED → GREEN
2. Write `test_remove_missing_raises_key_error` — RED → GREEN

### 10. Update `__init__.py`

1. Export `Corpus`, `CompressionPolicy`, `VectorEntry` from `corpus/__init__.py`
2. Run full tool suite

## Verification

```bash
ruff check . && ruff format --check .
mypy --strict .
pytest tests/corpus/ -v --cov=tinyquant/corpus --cov-fail-under=90
pytest tests/codec/ -v  # regression check
```

## Estimated scope

- ~4 source files created, 1 modified
- ~4 test files created
- ~54 tests
- ~500 lines of source + ~700 lines of tests

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-03-codec-service|Phase 3]]
- [[plans/phase-05-backend-layer|Phase 5: Backend Layer]]
