# rust/crates/tinyquant-bruteforce/src

Source modules for the `tinyquant-bruteforce` crate. All modules are
`pub(crate)` — only `BruteForceBackend` and `BackendError` are re-exported
from `lib.rs`.

## What lives here

| File | Role |
| --- | --- |
| `lib.rs` | Crate root; re-exports public surface and declares modules |
| `backend.rs` | `BruteForceBackend` struct and `SearchBackend` impl |
| `errors.rs` | `BackendError` enum |
| `similarity.rs` | Portable cosine-similarity kernel |
| `similarity_simd.rs` | SIMD-accelerated cosine kernel (compiled only with `--features simd`) |
| `store.rs` | In-memory vector store used by the backend |

## How this area fits the system

`backend.rs` is the integration point: it calls into `store.rs` to retrieve
vectors and into `similarity.rs` (or `similarity_simd.rs` when the feature
flag is active) to score them. The backend is consumed by the bench and
integration-test crates via the `SearchBackend` trait from `tinyquant-core`.

## Common edit paths

- Adding or tuning SIMD kernels: `similarity_simd.rs`
- Changing scoring logic: `similarity.rs` and corresponding tests in `../tests/`
- Error variants: `errors.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
