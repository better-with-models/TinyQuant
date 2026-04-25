# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-pgvector/tests`

Integration tests for `PgvectorAdapter`. Tests require a live PostgreSQL
instance with the `pgvector` extension installed. The `testcontainers/` helper
spins up an ephemeral Postgres container via Docker for CI; local runs can
point at a pre-existing instance via `DATABASE_URL`.

## What this area contains

- primary responsibility: validate `PgvectorAdapter` correctness against a real PostgreSQL + pgvector instance
- main entrypoints: `adapter.rs` (full adapter integration tests), `smoke.rs` (connection and schema smoke checks)
- common changes: updating tests when the SQL schema or wire format changes; adding new adapter test cases

## Layout

```text
tests/
├── fixtures/
├── testcontainers/
├── adapter.rs
├── README.md
└── smoke.rs
```

## Common workflows

### Update existing behavior

1. Read `adapter.rs` before changing a test — understand what invariant it is asserting.
2. Schema changes require updating `fixtures/0001_create_vectors_table.sql`.
3. Run `cargo test -p tinyquant-pgvector` with a live database to validate.

### Add a new test

1. Add the test to `adapter.rs` (integration) or `smoke.rs` (basic connectivity).
2. If new fixture data is needed, add files under `fixtures/` and commit them.
3. Update the layout section if new files are added.

## Invariants — Do Not Violate

- Do not mock the database; these tests must run against a real Postgres + pgvector instance.
- `fixtures/` files are committed golden data; do not regenerate silently.
- Tests must clean up created rows or use a dedicated test schema to avoid cross-test pollution.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
