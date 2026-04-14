# rust/crates/tinyquant-pgvector/tests/fixtures

Committed golden files for the `tinyquant-pgvector` integration test suite.

## What lives here

| File | Purpose |
| --- | --- |
| `0001_create_vectors_table.sql` | DDL migration that creates the `vectors` table with the `pgvector` column type; applied by test setup before each run |
| `pgvector_wire_100.json` | 100-vector golden sample in the pgvector binary wire format; used by `tests/adapter.rs` to assert round-trip byte stability |

## How this area fits the system

These files are the stable contract between the Rust adapter implementation and
the PostgreSQL schema. `adapter.rs` tests apply `0001_create_vectors_table.sql`
at setup time and compare stored bytes against `pgvector_wire_100.json`. Any
format or schema change must be paired with an explicit fixture update.

## Common edit paths

- **`0001_create_vectors_table.sql`** — update when the column type, index type, or table layout changes.
- **`pgvector_wire_100.json`** — regenerate when the binary wire format changes; never edit by hand.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
