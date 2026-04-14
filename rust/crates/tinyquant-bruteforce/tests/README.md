# rust/crates/tinyquant-bruteforce/tests

Integration tests for `tinyquant-bruteforce`. These run against the compiled
crate as a black box via `cargo test`, exercising the public `SearchBackend`
API without access to internal modules.

## What lives here

| File | Role |
|---|---|
| `backend.rs` | Full-cycle integration tests: ingest, search, edge cases |
| `smoke.rs` | Minimal sanity check that the crate links and returns a result |

There is also a `fixtures/` subdirectory containing data files loaded by
`backend.rs` tests.

## How this area fits the system

Tests here are the first line of defence for regressions in `BruteForceBackend`
behavior. The CI `cargo test` step runs these unconditionally; they have no
external service dependencies.

## Common edit paths

- New behavioral tests: `backend.rs`
- New test fixtures: `fixtures/`
- Build-level sanity: `smoke.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
