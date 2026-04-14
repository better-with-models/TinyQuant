# rust/crates/tinyquant-core/tests

Integration tests for `tinyquant-core`. Each `.rs` file at this level is a standalone Cargo integration test binary that exercises the public API. Shared deterministic input helpers live in the `common/` subdirectory. Binary fixture files consumed by parity tests live in `fixtures/`.

## What lives here

| File | What it covers |
| --- | --- |
| `smoke.rs` | Basic round-trip sanity |
| `codebook.rs` | `Codebook` construction and centroid lookup |
| `codec_config.rs` | `CodecConfig` validation and hash stability |
| `codec_service.rs` | `Codec` compress/decompress round-trip |
| `codec_fixture_parity.rs` | Regression against Rust-generated binary fixtures |
| `rotation_fixture_parity.rs` | Regression against Rust-generated rotation fixtures |
| `rotation_matrix.rs`, `rotation_cache.rs` | Rotation primitive properties |
| `quantize.rs`, `residual.rs` | Kernel-level correctness |
| `corpus_aggregate.rs`, `corpus_events.rs`, `corpus_insertion_order.rs` | `Corpus` domain rules |
| `compression_policy.rs`, `entry_meta_value.rs`, `vector_entry.rs` | Domain type invariants |
| `errors.rs`, `types.rs` | Error and type edge cases |
| `backend_trait.rs` | `SearchBackend` trait object safety |
| `batch_determinism.rs`, `batch_parallel.rs` | Batch compress determinism and parallelism contracts |
| `simd_dispatch_cache.rs` | Dispatch selection logic |
| `simd_parity_*.rs` | Per-kernel and cross-dispatch output parity (require `feature = "simd"`) |
| `compressed_vector.rs` | `CompressedVector` value object properties |
| `common/` | Shared deterministic input generators (included via `mod common`) |
| `fixtures/` | Pre-generated binary fixture files for parity tests |

## How this area fits the system

These tests are the primary correctness gate for the codec. CI runs them against the default feature set and again with `--features simd,avx512`. Parity tests (`codec_fixture_parity.rs`, `rotation_fixture_parity.rs`) lock in deterministic output so that refactors cannot silently change encoded bytes.

## Common edit paths

- **New codec feature** — add or extend an existing test file, or create a new one named after the feature
- **Updating fixture binaries** — run `cargo run -p tinyquant-core --example dump_codec_fixture` and replace files in `fixtures/`
- **SIMD parity failures** — check the appropriate `simd_parity_*.rs` and trace into `common/mod.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
