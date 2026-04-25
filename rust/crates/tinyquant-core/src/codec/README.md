# rust/crates/tinyquant-core/src/codec

The `codec` module is the compression pipeline: deterministic `CodecConfig`, canonical `RotationMatrix` and `RotationCache`, uniform-quantile `Codebook`, scalar quantize/dequantize kernels, FP16 residual helpers, the stateless `Codec` service (`compress`/`decompress`), SIMD dispatch, and `Parallelism` control. It is separated from `corpus` and `backend` because it has no knowledge of named vectors or search — it only transforms `f32` slices into `CompressedVector` values and back.

## What lives here

| File | Public items |
| --- | --- |
| `codec_config.rs` | `CodecConfig`, `SUPPORTED_BIT_WIDTHS` |
| `rotation_matrix.rs` | `RotationMatrix` |
| `rotation_cache.rs` | `RotationCache`, `DEFAULT_CAPACITY` |
| `gaussian.rs` | Gaussian sampling helpers for rotation generation |
| `codebook.rs` | `Codebook` (uniform-quantile centroid table) |
| `quantize.rs` | Scalar quantize/dequantize (crate-private) |
| `residual.rs` | FP16 residual encode/decode helpers |
| `compressed_vector.rs` | `CompressedVector` value object |
| `service.rs` | `Codec` service, `compress`, `decompress` free functions |
| `parallelism.rs` | `Parallelism` enum (serial / rayon-custom) |
| `dispatch.rs` | `DispatchKind` runtime SIMD selection (`feature = "simd"`) |
| `simd_api.rs` | Public SIMD kernel surface (`feature = "simd"`) |
| `batch.rs` | Batch compress helper (`feature = "std"`, crate-private) |
| `batch_error.rs` | Batch error type (`feature = "std"`, crate-private) |
| `kernels/` | Scalar, AVX2, AVX-512, NEON, portable kernel implementations |

## How this area fits the system

The `Corpus` aggregate (`../corpus/`) calls into the `Codec` service when a `VectorEntry` transitions from raw to compressed storage. `tinyquant-io` serializes the `CompressedVector` produced here. Backend crates dequantize vectors via `decompress` for distance computations.

All paths must remain `no_std + alloc` unless gated on `feature = "std"`. The `Codebook` centroid set is derived from training data and stored in `CompressedVector`; changing its layout is a breaking change for serialized corpora.

## Common edit paths

- **Kernel logic** — `kernels/scalar.rs` (canonical reference); then `kernels/avx2.rs`, `kernels/neon.rs`
- **Codec pipeline** — `service.rs`
- **Config parameters** — `codec_config.rs`
- **Residual encoding** — `residual.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
