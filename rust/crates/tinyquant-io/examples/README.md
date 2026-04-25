# rust/crates/tinyquant-io/examples

Runnable examples for the `tinyquant-io` serialization crate. Each example
demonstrates end-to-end usage of the file-format layer without requiring the
full Python binding stack.

## What lives here

| File | Purpose |
| --- | --- |
| `gen_corpus_fixture.rs` | Generate golden `.tqcv` fixture files from a seeded random corpus; used by the test suite to produce deterministic baselines under `tests/fixtures/` |

## How this area fits the system

Examples depend only on `tinyquant-io` and `tinyquant-core`. They are the
canonical demonstration of the public serialization API and are the first place
to look when debugging a fixture mismatch or validating a file-format change.
The generated fixtures are committed and consumed by integration tests in
`tests/roundtrip.rs` and `tests/zero_copy.rs`.

## Common edit paths

- **`gen_corpus_fixture.rs`** — update when the `.tqcv` format changes or when
  new fixture dimensions are needed for the test matrix.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
