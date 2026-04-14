# rust/crates/tinyquant-core/src/backend

The `backend` module defines the uniform search-backend protocol: the `SearchBackend` trait and the `SearchResult` type. Concrete implementations (`tinyquant-bruteforce`, `tinyquant-pgvector`) live in separate crates and implement this trait. Keeping the trait in `tinyquant-core` lets the corpus layer reference it without depending on any concrete backend.

## What lives here

| File | Public items |
| --- | --- |
| `protocol.rs` | `SearchBackend` trait, `SearchResult` |
| `mod.rs` | Re-exports `protocol::{SearchBackend, SearchResult}` |

## How this area fits the system

Backend crates implement `SearchBackend` and are registered with a `Corpus` at runtime. The `prelude` re-exports both items so downstream code can do `use tinyquant_core::prelude::*` without importing individual paths.

This module has no dependencies within the crate other than `types.rs` and `errors.rs`. It is intentionally thin — all search logic is in the concrete backend crates.

## Common edit paths

- **Adding methods to the backend contract** — `protocol.rs`; all concrete backend crates must then be updated to implement the new method

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
