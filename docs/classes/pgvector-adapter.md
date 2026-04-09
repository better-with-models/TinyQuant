---
title: PgvectorAdapter
tags:
  - classes
  - backend
  - adapter
date-created: 2026-04-08
status: active
category: design
---

# PgvectorAdapter

> [!info] Responsibility
> Anti-corruption layer that translates between TinyQuant's vector types and
> pgvector's wire format. Implements the [[classes/search-backend|SearchBackend]]
> protocol for PostgreSQL + pgvector deployments.

## Location

```text
src/tinyquant_cpu/backend/adapters/pgvector.py
```

## Category

**Adapter (ACL)** — implements `SearchBackend` by translating to/from
pgvector-specific formats.

## Constructor

| Parameter | Type | Description |
|-----------|------|-------------|
| `connection_factory` | `Callable[[], Connection]` | Factory that provides a database connection (DIP: no direct psycopg import in domain) |
| `table_name` | `str` | Target table for vector storage |
| `dimension` | `int` | Expected vector dimensionality |

## Methods

### `search`

```python
def search(
    self,
    query: NDArray[np.float32],
    top_k: int,
) -> list[SearchResult]:
```

**Purpose:** Execute a pgvector similarity query and return ranked results.

**Implementation:** translates the NumPy array to a pgvector-compatible
literal, executes the SQL query, and maps results to `SearchResult` objects.

---

### `ingest`

```python
def ingest(
    self,
    vectors: dict[str, NDArray[np.float32]],
) -> None:
```

**Purpose:** Insert or upsert vectors into the pgvector table.

**Implementation:** converts NumPy arrays to pgvector array literals and
executes batch inserts.

---

### `remove`

```python
def remove(
    self,
    vector_ids: Sequence[str],
) -> None:
```

**Purpose:** Delete vectors from the pgvector table by ID.

## Invariants

1. The adapter never exposes compressed representations to PostgreSQL
2. All SQL uses parameterized queries (no string interpolation — OWASP)
3. The connection factory is the only infrastructure dependency — passed in,
   not imported

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Implements | [[classes/search-backend\|SearchBackend]] | Satisfies the protocol structurally |
| Returns | [[classes/search-result\|SearchResult]] | Mapped from SQL result rows |
| Depends on | `connection_factory` (injected) | DIP: infrastructure behind a callable |

## Implementation notes

> [!warning] External dependency
> This adapter requires `psycopg` (or `psycopg2`) and a running PostgreSQL
> instance with the pgvector extension. It is an optional dependency — not
> required for core TinyQuant functionality.

## Test file

```text
tests/backend/test_pgvector_adapter.py  (integration test; requires PostgreSQL)
```

## See also

- [[classes/search-backend|SearchBackend]]
- [[classes/brute-force-backend|BruteForceBackend]]
- [[design/architecture/low-coupling|Low Coupling]]
