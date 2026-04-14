# tests/reference/tinyquant_py_reference/backend/adapters

This directory contains the `adapters` sub-package of the reference
implementation's `backend` layer. It houses `PgvectorAdapter`, the anti-corruption
layer that translates TinyQuant vectors into the pgvector wire format and executes
cosine similarity queries against a live PostgreSQL instance. The adapter is kept
in a dedicated sub-package because it has an optional runtime dependency on
`psycopg` and `pgvector` that the rest of the backend layer (`BruteForceBackend`,
`SearchResult`) does not share.

## What lives here

- `pgvector.py` — `PgvectorAdapter` class implementing the `SearchBackend`
  protocol for PostgreSQL + pgvector:
  - `__init__` accepts a `connection_factory` callable (dependency inversion),
    `table_name`, and `dimension`.
  - `ingest` performs an upsert (`INSERT … ON CONFLICT DO UPDATE`) for each
    vector using parameterized queries; vectors are passed as Python lists
    (`vector.tolist()`).
  - `search` executes a cosine-distance query (`<=>` operator) and converts rows
    to `SearchResult` objects sorted by descending similarity score.
  - `remove` deletes rows by ID using `ANY(%s)` with a list parameter.
  - All SQL uses parameterized values for data; `table_name` is a developer-
    controlled identifier interpolated directly into the SQL string (noted with
    `# noqa: S608` comments).
- `__init__.py` — exports `PgvectorAdapter` in `__all__`.

## How this area fits the system

`PgvectorAdapter` is exercised by `tests/integration/test_pgvector.py` (EB-02),
which provisions a real PostgreSQL database via `PGVECTOR_TEST_DSN` or
testcontainers. The adapter depends only on `tinyquant_py_reference.backend.protocol`
(`SearchResult`) and has no dependency on the codec or corpus layers. The
`connection_factory` callable is injected at construction time so that tests can
supply a pre-configured session connection instead of opening a new one.

## Common edit paths

- `pgvector.py` — update `ingest` if a different conflict-resolution strategy is
  needed (e.g., skip on conflict rather than update).
- `pgvector.py` — update `search` if the distance metric changes (e.g., inner
  product `<#>` instead of cosine `<=>`) or if the result format changes.
- `__init__.py` — add new adapter classes here when additional backends
  (e.g., Qdrant, Weaviate) are implemented.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
