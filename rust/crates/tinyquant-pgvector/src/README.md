# rust/crates/tinyquant-pgvector/src

Source files for the `tinyquant-pgvector` crate implementing the PostgreSQL +
pgvector storage backend.

## What lives here

| File | Purpose |
| --- | --- |
| `lib.rs` | Crate root; re-exports the public API surface |
| `adapter.rs` | `PgvectorAdapter` struct and `Backend` trait implementation |
| `errors.rs` | `PgvectorError` enum with variants for connection, query, and wire errors |
| `sql.rs` | Parameterised SQL strings for insert, search, and schema operations |
| `wire.rs` | Byte-level serialization of compressed vectors for the pgvector wire format |

## How this area fits the system

`PgvectorAdapter` is plugged into `Corpus` as a `Backend` implementation.
`sql.rs` owns all query strings; `wire.rs` converts between `CompressedVector`
and the `vector` column type expected by pgvector. `errors.rs` maps
database-level errors to the `Backend` trait error type.

## Common edit paths

- **`sql.rs`** — update when the schema or search query changes.
- **`wire.rs`** — update when the pgvector binary wire format changes.
- **`adapter.rs`** — update when the `Backend` trait interface evolves.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
