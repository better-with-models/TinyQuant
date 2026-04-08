---
title: "Phase 9: Pgvector Adapter"
tags:
  - plans
  - phase-9
  - backend
  - pgvector
  - adapter
date-created: 2026-04-08
status: pending
category: planning
---

# Phase 9: Pgvector Adapter

> [!info] Goal
> Implement the `PgvectorAdapter` anti-corruption layer for PostgreSQL +
> pgvector integration. This is an optional backend — core TinyQuant works
> without it.

## Prerequisites

- [[plans/phase-08-ci-cd-workflows|Phase 8]] complete (CI pipeline running)

## Deliverables

| File | Class | Spec |
|------|-------|------|
| `src/tinyquant/backend/adapters/pgvector.py` | `PgvectorAdapter` | [[classes/pgvector-adapter\|PgvectorAdapter]] |
| `src/tinyquant/backend/adapters/__init__.py` | Re-export adapter | — |
| `tests/integration/test_pgvector.py` | ~6 integration tests | [[qa/integration-plan/README\|Integration Plan EB-02]] |

## Steps (TDD order)

### 1. Adapter skeleton

1. Write `PgvectorAdapter` class with `__init__` accepting `connection_factory`, `table_name`, `dimension`
2. Implement `SearchBackend` protocol methods as stubs

### 2. ingest

1. Write `test_ingest_writes_to_database` — RED (requires test database)
2. Implement `ingest()` with parameterized INSERT/UPSERT — GREEN
3. Verify SQL uses parameterized queries (no f-string interpolation)

### 3. search

1. Write `test_search_returns_ranked_results` — RED
2. Implement `search()` with pgvector `<=>` cosine distance operator — GREEN
3. Write `test_dimension_preserved_through_wire_format` — RED → GREEN

### 4. remove

1. Write `test_remove_deletes_from_database` — RED → GREEN

### 5. Security

1. Write `test_parameterized_queries_no_injection` — verify parameterized queries
2. Write `test_connection_factory_is_called` — verify DIP compliance

### 6. Test infrastructure

1. Add `pgvector` pytest mark that skips when `PGVECTOR_TEST_DSN` is not set
2. Add `pgvector_connection` session fixture
3. Add table setup/teardown in fixture

## Verification

```bash
ruff check . && ruff format --check .
mypy --strict .
# Without PostgreSQL:
pytest tests/ --ignore=tests/integration/test_pgvector.py
# With PostgreSQL:
PGVECTOR_TEST_DSN="postgresql://..." pytest tests/integration/test_pgvector.py -v
```

## Estimated scope

- ~2 source files created/modified
- ~1 test file created
- ~6 tests
- ~150 lines of source + ~150 lines of tests

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-08-ci-cd-workflows|Phase 8]]
- [[plans/phase-10-calibration-release|Phase 10: Calibration & Release]]
