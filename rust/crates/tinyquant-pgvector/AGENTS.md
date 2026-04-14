# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-pgvector`

This crate provides `PgvectorAdapter`, a PostgreSQL storage backend that
persists compressed vectors via the `pgvector` extension. It implements the
`Backend` trait from `tinyquant-core` so that a `Corpus` can store and search
embeddings in a live Postgres instance without any Python dependency.

## What this area contains

- primary responsibility: PostgreSQL + pgvector storage backend implementing the `Backend` trait
- main entrypoints: `src/adapter.rs` (the adapter struct and `Backend` impl), `src/lib.rs` (crate root)
- common changes: updating SQL when the schema evolves; adding connection-pool configuration options

## Layout

```text
tinyquant-pgvector/
├── src/
├── tests/
├── Cargo.toml
└── README.md
```

## Common workflows

### Update existing behavior

1. Read `src/adapter.rs` before touching the SQL or wire-format logic.
2. Schema changes require updating `tests/fixtures/0001_create_vectors_table.sql` and regenerating `tests/fixtures/pgvector_wire_100.json`.
3. Run `cargo test -p tinyquant-pgvector` against a live PostgreSQL + pgvector instance.

### Add a new configuration option

1. Add the field to the adapter config struct in `src/adapter.rs`.
2. Update `src/lib.rs` re-exports if the new type is part of the public API.
3. Document the option with a `///` comment.

## Invariants — Do Not Violate

- This crate requires a live PostgreSQL + pgvector instance; do not mock the database in integration tests.
- All SQL is parameterised; never interpolate user-supplied strings into query strings.
- The wire format in `tests/fixtures/pgvector_wire_100.json` is a committed golden file; regenerate explicitly, not automatically.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
