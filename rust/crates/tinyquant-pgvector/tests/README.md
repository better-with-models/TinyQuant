# rust/crates/tinyquant-pgvector/tests

Integration test suite for `PgvectorAdapter`. Requires a live PostgreSQL
instance with the `pgvector` extension installed.

## What lives here

| File / Directory | Purpose |
| --- | --- |
| `fixtures/` | Committed SQL migrations and JSON wire-format golden files |
| `testcontainers/` | Docker-based ephemeral Postgres helper used in CI |
| `adapter.rs` | Full integration tests for insert, search, and round-trip correctness |
| `smoke.rs` | Basic connection and schema smoke checks |

## How this area fits the system

These tests are the primary quality gate for `PgvectorAdapter`. CI runs them
against a Docker-hosted Postgres + pgvector instance via `testcontainers/`.
Local runs can target a pre-existing instance by setting `DATABASE_URL`.

## Common edit paths

- **`adapter.rs`** — add new test cases when adapter behaviour changes.
- **`fixtures/`** — update SQL or JSON files when the schema or wire format evolves.
- **`testcontainers/`** — update the container image tag when the pgvector version is bumped.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
