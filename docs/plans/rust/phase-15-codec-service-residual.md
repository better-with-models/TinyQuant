---
title: "Phase 15: Codec Service, Residual, and CompressedVector (in-memory)"
tags:
  - plans
  - rust
  - phase-15
  - codec
  - residual
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 15: Codec Service, Residual, and CompressedVector (in-memory)

> [!info] Goal
> Implement the stateless `Codec` service, the FP16 residual
> helpers, and the in-memory `CompressedVector` value object.
> Serialization lives in phase 16; this phase produces and consumes
> `CompressedVector`s as opaque in-memory values only.

> [!note] Reference docs
> - [[design/rust/type-mapping|Type Mapping]] §Codec service, §CompressedVector
> - [[design/rust/numerical-semantics|Numerical Semantics]] §fp16, §Rotation application
> - [[design/rust/memory-layout|Memory Layout]] §CompressedVector layout

## Prerequisites

- Phase 14 complete (codebook + scalar quantize kernels landed).

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-core/src/codec/residual.rs` | `compute_residual`, `apply_residual` |
| `rust/crates/tinyquant-core/src/codec/compressed_vector.rs` | `CompressedVector` in-memory type |
| `rust/crates/tinyquant-core/src/codec/codec.rs` | `Codec` service + free fns |
| `rust/crates/tinyquant-core/src/codec/parallelism.rs` | `Parallelism` enum |
| `rust/crates/tinyquant-core/tests/residual.rs` | Residual round-trip |
| `rust/crates/tinyquant-core/tests/compressed_vector.rs` | Value-object invariants |
| `rust/crates/tinyquant-core/tests/codec.rs` | Full compress/decompress |
| `rust/crates/tinyquant-core/tests/fixtures/codec/...` | Gold fixtures |

## Steps (TDD order)

- [ ] **Step 1: Residual round-trip test**

```rust
#[test]
fn compute_and_apply_residual_round_trip_is_bounded() {
    let original = vec![0.12_f32, -0.34, 0.56, -0.78];
    let reconstructed = vec![0.10_f32, -0.30, 0.60, -0.80];
    let residual = tinyquant_core::codec::residual::compute_residual(&original, &reconstructed);
    assert_eq!(residual.len(), original.len() * 2);
    let mut out = reconstructed.clone();
    tinyquant_core::codec::residual::apply_residual(&mut out, &residual).unwrap();
    for (r, o) in out.iter().zip(original.iter()) {
        assert!((r - o).abs() < 1e-2);
    }
}
```

- [ ] **Step 2: Implement `residual.rs`**

```rust
use crate::errors::CodecError;
use alloc::vec::Vec;
use half::f16;

pub fn compute_residual(original: &[f32], reconstructed: &[f32]) -> Vec<u8> {
    debug_assert_eq!(original.len(), reconstructed.len());
    let mut out = Vec::with_capacity(original.len() * 2);
    for (o, r) in original.iter().zip(reconstructed.iter()) {
        let diff = f16::from_f32(o - r);
        out.extend_from_slice(&diff.to_le_bytes());
    }
    out
}

pub fn apply_residual(values: &mut [f32], residual: &[u8]) -> Result<(), CodecError> {
    if residual.len() != values.len() * 2 {
        return Err(CodecError::LengthMismatch {
            left: residual.len(),
            right: values.len() * 2,
        });
    }
    for (i, chunk) in residual.chunks_exact(2).enumerate() {
        let bits = u16::from_le_bytes([chunk[0], chunk[1]]);
        values[i] += f16::from_bits(bits).to_f32();
    }
    Ok(())
}
```

- [ ] **Step 3: Run residual test — expect pass.**

- [ ] **Step 4: Property test for residual fp16 parity vs Python**

Use a fixture: 1 000 f32 pairs generated in Python and their
`np.float16` bytes captured. Assert Rust's `compute_residual`
produces byte-identical output.

- [ ] **Step 5: Write failing `tests/compressed_vector.rs`**

Covers:

- Construction success with matching dimension and indices length.
- Rejection of mismatched dimension.
- Rejection of unsupported bit width.
- `has_residual` flag.
- `size_bytes` matches Python formula:
  `ceil(dim * bit_width / 8) + residual.len()`.

- [ ] **Step 6: Implement `compressed_vector.rs`**

The in-memory-only variant (no `to_bytes`/`from_bytes` — those
arrive in phase 16). Exact signatures from
[[design/rust/type-mapping|Type Mapping]].

- [ ] **Step 7: Run — expect pass.**

- [ ] **Step 8: Implement `parallelism.rs`**

```rust
#[derive(Clone, Copy, Default)]
pub enum Parallelism {
    #[default]
    Serial,
    Custom(fn(count: usize, body: &(dyn Fn(usize) + Sync + Send))),
}

impl Parallelism {
    pub fn for_each_row<F>(self, count: usize, body: F)
    where
        F: Fn(usize) + Sync + Send,
    {
        match self {
            Self::Serial => (0..count).for_each(body),
            Self::Custom(driver) => driver(count, &body),
        }
    }
}
```

- [ ] **Step 9: Write failing `tests/codec.rs` — single vector round trip**

```rust
#[test]
fn compress_then_decompress_recovers_vector_within_bound() {
    let training = load_training_fixture("fixtures/codec/training_n10000_d64.f32.bin");
    let config = CodecConfig::new(4, 42, 64, true).unwrap();
    let codebook = Codebook::train(&training, 10_000, 64, &config).unwrap();
    let codec = Codec::new();

    let vector = &training[0..64];
    let cv = codec.compress(vector, &config, &codebook).unwrap();
    assert_eq!(cv.config_hash().as_ref(), config.config_hash().as_ref());
    assert_eq!(cv.bit_width(), 4);
    assert!(cv.has_residual());

    let recovered = codec.decompress(&cv, &config, &codebook).unwrap();
    assert_eq!(recovered.len(), 64);
    let mse: f32 = vector.iter().zip(recovered.iter())
        .map(|(a, b)| (a - b).powi(2)).sum::<f32>() / 64.0;
    assert!(mse < 1e-2, "mse={mse}");
}
```

- [ ] **Step 10: Implement `codec.rs`**

Pipeline (following Python exactly):

```
compress:
  1. validate (dim, codebook bit width)
  2. rotation = RotationMatrix::from_config(&config)  (via cache)
  3. rotated := apply rotation
  4. indices := codebook.quantize(rotated)
  5. if residual_enabled:
       reconstructed := codebook.dequantize(indices)
       residual := compute_residual(rotated, reconstructed)
  6. return CompressedVector::new(...)

decompress:
  1. validate (config hash match, bit width match)
  2. values := codebook.dequantize(indices)
  3. if has_residual: apply_residual(values, residual)
  4. rotation := RotationMatrix::from_config(&config)
  5. return rotation.apply_inverse(values)
```

- [ ] **Step 11: Run — expect pass.**

- [ ] **Step 12: Round-trip determinism test**

```rust
#[test]
fn compress_is_deterministic_across_calls() {
    // Same inputs → identical CompressedVector fields.
}
```

- [ ] **Step 13: Error-path tests**

- `DimensionMismatch` when input length != config.dimension.
- `CodebookIncompatible` when codebook bit width mismatches.
- `ConfigMismatch` when decompress uses a different config.

- [ ] **Step 14: Batch compress/decompress with Parallelism::Serial**

```rust
#[test]
fn compress_batch_matches_individual_calls() {
    // For 16 vectors: batch vs per-vector compression → identical
    // CompressedVector bytes (in-memory indices + residual).
}
```

- [ ] **Step 15: Implement batch methods**

Serial path only in this phase; parallel path lands in phase 21.

- [ ] **Step 16: Add free functions `compress` / `decompress`**

```rust
pub fn compress(
    vector: &[f32],
    config: &CodecConfig,
    codebook: &Codebook,
) -> Result<CompressedVector, CodecError> {
    Codec::new().compress(vector, config, codebook)
}
```

And the symmetric `decompress`.

- [ ] **Step 17: Update `prelude` and re-exports**

```rust
// prelude.rs
pub use crate::codec::{
    Codebook, Codec, CodecConfig, CompressedVector, RotationMatrix,
    compress, decompress,
};
```

- [ ] **Step 18: Score fidelity test**

Using a 1 000-vector fixture, assert Pearson ρ ≥ 0.99 on pairwise
cosine similarities pre- and post-codec. This is a coarse gate;
full calibration lands in phase 21.

- [ ] **Step 19: Run workspace test/clippy/fmt.**

- [ ] **Step 20: Commit**

```bash
git add rust/crates/tinyquant-core
git commit -m "feat(tinyquant-core): add Codec service, residual, and CompressedVector"
```

## Acceptance criteria

- `Codec::compress` + `Codec::decompress` round trip on the gold
  fixture has MSE < 1e-2.
- Python-parity residual bytes match on the f16 fixture.
- Batch compress equals per-vector compress for serial parallelism.
- `ConfigMismatch`, `DimensionMismatch`, `CodebookIncompatible`
  errors trip at the same points as the Python reference.
- Pearson ρ ≥ 0.99 on the 1 000-vector fixture.
- `no_std` build still green.
- Clippy + fmt clean.

## Out of scope

- Serialization (`to_bytes` / `from_bytes`) — phase 16.
- Parallel batch paths — phase 21.
- SIMD kernels — phase 20.
- Corpus aggregate — phase 18.

## See also

- [[plans/rust/phase-14-codebook-quantize|Phase 14]]
- [[plans/rust/phase-16-serialization-parity|Phase 16]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/memory-layout|Memory Layout]]
