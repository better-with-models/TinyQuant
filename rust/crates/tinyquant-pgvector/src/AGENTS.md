# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-pgvector/src`

Source module for the `tinyquant-pgvector` crate. Contains the adapter
implementation, SQL helpers, wire-format serialization, and error types for the
PostgreSQL + pgvector backend.

## What this area contains

- primary responsibility: implement `PgvectorAdapter` and all supporting types for Postgres-backed vector storage
- main entrypoints: `adapter.rs` (core logic and `Backend` impl), `lib.rs` (public API surface)
- common changes: SQL query updates in `sql.rs`; wire-format changes in `wire.rs`; new error variants in `errors.rs`

## Layout

```text
src/
├── adapter.rs
├── errors.rs
├── lib.rs
├── README.md
├── sql.rs
└── wire.rs
```

## Common workflows

### Update existing behavior

1. Read `adapter.rs` first to understand the `Backend` impl before touching `sql.rs` or `wire.rs`.
2. Any SQL change must be paired with an updated fixture in `tests/fixtures/`.
3. New error variants in `errors.rs` must implement `std::error::Error` and map cleanly through the `Backend` trait error type.

### Add a new file or module

1. Register the new module in `lib.rs`.
2. Update the layout section above.
3. Add `///` doc comments on every public type and function.

## Invariants — Do Not Violate

- All SQL must use parameterised queries via `tokio-postgres` or `sqlx` bind parameters.
- `wire.rs` encoding must remain byte-compatible with the `pgvector_wire_100.json` golden file.
- Never expose internal connection state through the public `Backend` trait surface.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
