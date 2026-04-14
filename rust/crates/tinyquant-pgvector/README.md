# rust/crates/tinyquant-pgvector

`tinyquant-pgvector` provides `PgvectorAdapter`, an implementation of the
`SearchBackend` trait that stores and queries vectors in a PostgreSQL database
with the `pgvector` extension installed. It is kept separate from the
brute-force backend so the PostgreSQL dependency is not pulled into builds that
do not need it.

## What lives here

| Path | Role |
| --- | --- |
| `src/lib.rs` | Crate root; re-exports `PgvectorAdapter`, `BackendError`, and core traits |
| `src/adapter.rs` | `PgvectorAdapter` struct and `SearchBackend` impl |
| `src/errors.rs` | `BackendError` enum for this crate |
| `src/sql.rs` | SQL query strings / helpers |
| `src/wire.rs` | Wire-format conversions between Rust types and pgvector binary format |

## How this area fits the system

The adapter is an alternative drop-in for `BruteForceBackend` wherever a
`SearchBackend` is expected. Tests that require a live PostgreSQL server are
gated behind the `test-containers` feature flag and are not run by the default
`cargo test` invocation.

## Common edit paths

- SQL schema or query changes: `src/sql.rs`
- Wire-format changes: `src/wire.rs`
- Error variants: `src/errors.rs`
- Live-DB integration tests: run with `cargo test --features test-containers`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
