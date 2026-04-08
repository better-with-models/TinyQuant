---
title: "Phase 5: Backend Layer"
tags:
  - plans
  - phase-5
  - backend
  - protocol
date-created: 2026-04-08
status: pending
category: planning
---

# Phase 5: Backend Layer

> [!info] Goal
> Implement the backend protocol context: `SearchBackend` protocol,
> `SearchResult` value object, and `BruteForceBackend` reference
> implementation. This completes the three bounded contexts.

## Prerequisites

- [[plans/phase-04-corpus-layer|Phase 4]] complete (corpus insert/decompress works)

## Deliverables

| File | Class | Spec |
|------|-------|------|
| `src/tinyquant/backend/protocol.py` | `SearchBackend`, `SearchResult` | [[classes/search-backend\|SearchBackend]], [[classes/search-result\|SearchResult]] |
| `src/tinyquant/backend/brute_force.py` | `BruteForceBackend` | [[classes/brute-force-backend\|BruteForceBackend]] |
| `src/tinyquant/backend/__init__.py` | Public re-exports | — |
| `tests/backend/test_brute_force.py` | ~12 tests | [[qa/unit-tests/test-brute-force\|Test spec]] |

## Steps (TDD order)

### 1. SearchResult value object

1. Write test for `SearchResult` construction and ordering — RED
2. Implement `SearchResult` frozen dataclass with `__lt__` — GREEN

### 2. SearchBackend protocol

1. Write `protocol.py` with `SearchBackend(Protocol)` class
2. No direct tests — verified through implementation conformance

### 3. BruteForceBackend — ingest

1. Write `test_ingest_stores_vectors` — RED
2. Implement `BruteForceBackend` with in-memory dict storage — GREEN
3. Write `test_ingest_duplicate_id_overwrites` — RED → GREEN

### 4. BruteForceBackend — search

1. Write `test_search_returns_top_k_results` — RED
2. Implement `search()` with cosine similarity — GREEN
3. Write `test_search_results_ranked_by_similarity` — RED → GREEN
4. Write `test_search_identical_vector_has_score_near_one` — RED → GREEN
5. Write `test_search_empty_backend_returns_empty` — RED → GREEN

### 5. BruteForceBackend — remove and clear

1. Write `test_remove_decreases_count` — RED → GREEN
2. Write `test_clear_empties_backend` — RED → GREEN

### 6. Update `__init__.py`

1. Export `SearchBackend`, `SearchResult`, `BruteForceBackend`
2. Update `tinyquant/__init__.py` top-level re-exports if desired
3. Run full tool suite

## Verification

```bash
ruff check . && ruff format --check .
mypy --strict .
pytest tests/backend/ -v --cov=tinyquant/backend --cov-fail-under=80
pytest tests/codec/ tests/corpus/ -v  # regression check
```

## Estimated scope

- ~2 source files created, 1 modified
- ~1 test file created
- ~12 tests
- ~150 lines of source + ~200 lines of tests

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-04-corpus-layer|Phase 4]]
- [[plans/phase-06-serialization|Phase 6: Serialization]]
