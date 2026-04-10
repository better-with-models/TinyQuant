---
title: "Phase 13: Rotation Matrix and Numerical Foundations"
tags:
  - plans
  - rust
  - phase-13
  - rotation
  - numerics
date-created: 2026-04-10
status: draft
category: planning
---

# Phase 13: Rotation Matrix and Numerical Foundations

> [!info] Goal
> Implement `CodecConfig` (with Python-parity `config_hash`),
> `RotationMatrix` (with the canonical `ChaChaGaussianStream` +
> `faer::qr` + sign correction pipeline), and the LRU-style
> `RotationCache`. Land the first Python-parity fixtures and prove
> effect-level parity against Python-generated rotations.

> [!note] Reference docs
> - [[design/rust/numerical-semantics|Numerical Semantics]]
> - [[design/rust/type-mapping|Type Mapping]]
> - [[design/rust/memory-layout|Memory Layout]]
> - [[design/rust/risks-and-mitigations|Risks and Mitigations]] R1–R3

## Prerequisites

- Phase 12 complete (shared types and errors in place).
- `faer = "0.19"`, `rand_chacha = "0.3"` added as workspace deps.

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-core/src/codec/mod.rs` | Codec submodule root |
| `rust/crates/tinyquant-core/src/codec/codec_config.rs` | `CodecConfig` |
| `rust/crates/tinyquant-core/src/codec/rotation_matrix.rs` | `RotationMatrix` |
| `rust/crates/tinyquant-core/src/codec/rotation_cache.rs` | Shared LRU cache |
| `rust/crates/tinyquant-core/src/codec/gaussian.rs` | `ChaChaGaussianStream` |
| `rust/crates/tinyquant-core/tests/codec_config.rs` | Unit tests |
| `rust/crates/tinyquant-core/tests/rotation_matrix.rs` | Unit tests |
| `rust/crates/tinyquant-core/tests/fixtures/config_hashes.json` | Python-parity hash fixtures |
| `rust/crates/tinyquant-core/tests/fixtures/rotation/seed_42_dim_64.f64.bin` | f64 rotation fixture |
| `rust/crates/tinyquant-core/tests/fixtures/rotation/seed_42_dim_768.f64.bin` | f64 rotation fixture |

### Fixture generation

`xtask fixtures refresh-rotation` is implemented in this phase. It:

1. Starts an in-process Python subprocess that imports
   `tinyquant_cpu.codec.rotation_matrix.RotationMatrix` and generates
   the matrix for each `(seed, dimension)` in the fixture list.
2. Writes the f64 bytes little-endian to the target `.bin` file.
3. Also generates the JSON of expected `config_hash` values for the
   120-triple canonical set.

The fixture files are committed to Git LFS.

## Steps (TDD order)

- [ ] **Step 1: Write failing `tests/codec_config.rs`**

Covers:

- Valid construction at every supported bit width.
- Rejection of unsupported bit width with exact error variant.
- Rejection of zero dimension.
- `num_codebook_entries` returns `2^bit_width`.
- `config_hash` matches the Python fixture for a known triple
  (`bit_width=4, seed=42, dim=768, residual=true`).
- Equality/hash parity: two configs with identical fields are `==`
  and have the same `Hash` output.

```rust
#[test]
fn config_hash_matches_python_fixture_bw4_seed42_dim768_res_on() {
    let c = CodecConfig::new(4, 42, 768, true).unwrap();
    let expected = include_str!("fixtures/config_hashes/bw4_s42_d768_rtrue.txt").trim();
    assert_eq!(c.config_hash().as_ref(), expected);
}
```

- [ ] **Step 2: Run — expect failure**

```bash
cargo test -p tinyquant-core --test codec_config
```

- [ ] **Step 3: Implement `codec_config.rs`**

Use the exact canonical-string format from the Python
implementation:

```rust
fn compute_hash(bit_width: u8, seed: u64, dimension: u32, residual_enabled: bool) -> Arc<str> {
    use sha2::{Digest, Sha256};
    let canonical = alloc::format!(
        "CodecConfig(bit_width={b},seed={s},dimension={d},residual_enabled={r})",
        b = bit_width,
        s = seed,
        d = dimension,
        r = if residual_enabled { "True" } else { "False" },
    );
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    Arc::from(hex::encode(hasher.finalize()))
}
```

(See [[design/rust/type-mapping|Type Mapping]] for the full
`CodecConfig` code.)

- [ ] **Step 4: Run fixture script to capture Python hash**

```bash
cargo xtask fixtures refresh-hashes
```

This generates `config_hashes/bw4_s42_d768_rtrue.txt` from the
Python reference. Commit the fixture.

- [ ] **Step 5: Run `cargo test -p tinyquant-core --test codec_config`** — expect pass.

- [ ] **Step 6: Exhaustive hash parity**

Add a proptest iterating over 120 canonical triples
`bit_width ∈ {2,4,8}`, `seed ∈ {0, 1, 42, 999, u64::MAX}`,
`dimension ∈ {1, 32, 768, 1536}`, `residual ∈ {true, false}`.
Load the JSON fixture `fixtures/config_hashes.json` and assert
every `(triple, hash)` pair matches.

- [ ] **Step 7: Write failing `tests/rotation_matrix.rs`**

Tests:

- `from_config(&config)` returns a matrix of correct dimension.
- The matrix is orthogonal within `1e-12`.
- Apply + apply_inverse is identity within `1e-5` on f32 inputs.
- Determinism: two calls with the same config return bit-identical
  f64 matrices.
- Parity: the f64 bytes for `(seed=42, dim=64)` match the fixture
  file within `1e-12` absolute tolerance.
- The orthogonality check passes after loading the fixture.

- [ ] **Step 8: Run — expect failure**

- [ ] **Step 9: Implement `gaussian.rs`**

```rust
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;

/// Deterministic standard-normal f64 stream, used for rotation matrix
/// generation. See docs/design/rust/numerical-semantics.md §R1.
pub(crate) struct ChaChaGaussianStream {
    rng: ChaCha20Rng,
    /// The cached second value from the previous Box-Muller draw.
    spare: Option<f64>,
}

impl ChaChaGaussianStream {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha20Rng::seed_from_u64(seed),
            spare: None,
        }
    }

    pub fn next_f64(&mut self) -> f64 {
        if let Some(s) = self.spare.take() {
            return s;
        }
        // Box-Muller: draw two uniforms, emit two normals.
        loop {
            let u1 = self.next_uniform();
            let u2 = self.next_uniform();
            if u1 <= 0.0 { continue; }
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * core::f64::consts::PI * u2;
            let z0 = r * theta.cos();
            let z1 = r * theta.sin();
            self.spare = Some(z1);
            return z0;
        }
    }

    fn next_uniform(&mut self) -> f64 {
        use rand_chacha::rand_core::RngCore;
        let n = self.rng.next_u64();
        // 53 bits of mantissa precision -> [0, 1).
        ((n >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }
}
```

- [ ] **Step 10: Implement `rotation_matrix.rs`**

```rust
use crate::codec::codec_config::CodecConfig;
use crate::codec::gaussian::ChaChaGaussianStream;
use crate::errors::CodecError;
use alloc::sync::Arc;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub struct RotationMatrix {
    matrix: Arc<[f64]>,
    seed: u64,
    dimension: u32,
}

impl RotationMatrix {
    pub fn from_config(config: &CodecConfig) -> Self {
        Self::build(config.seed(), config.dimension())
    }

    pub fn build(seed: u64, dimension: u32) -> Self {
        let dim = dimension as usize;
        let mut data = alloc::vec![0.0f64; dim * dim];
        let mut stream = ChaChaGaussianStream::new(seed);
        for i in 0..dim {
            for j in 0..dim {
                data[i * dim + j] = stream.next_f64();
            }
        }
        // QR decomposition via faer.
        let q_col_major = faer_qr_and_correct(&data, dim);
        // Store row-major.
        let mut row_major = alloc::vec![0.0f64; dim * dim];
        for i in 0..dim {
            for j in 0..dim {
                row_major[i * dim + j] = q_col_major[j * dim + i];
            }
        }
        Self {
            matrix: Arc::from(row_major),
            seed,
            dimension,
        }
    }

    #[inline]
    pub fn matrix(&self) -> &[f64] { &self.matrix }

    #[inline]
    pub const fn dimension(&self) -> u32 { self.dimension }

    pub fn apply_into(&self, input: &[f32], output: &mut [f32]) -> Result<(), CodecError> {
        let dim = self.dimension as usize;
        if input.len() != dim || output.len() != dim {
            return Err(CodecError::LengthMismatch { left: input.len(), right: output.len() });
        }
        // Promote to f64, multiply, cast back.
        let mut sum = alloc::vec![0.0f64; dim];
        for i in 0..dim {
            let row = &self.matrix[i * dim..(i + 1) * dim];
            let mut acc = 0.0f64;
            for j in 0..dim {
                acc += row[j] * f64::from(input[j]);
            }
            sum[i] = acc;
        }
        for i in 0..dim {
            output[i] = sum[i] as f32;
        }
        Ok(())
    }

    pub fn apply_inverse_into(&self, input: &[f32], output: &mut [f32]) -> Result<(), CodecError> {
        let dim = self.dimension as usize;
        if input.len() != dim || output.len() != dim {
            return Err(CodecError::LengthMismatch { left: input.len(), right: output.len() });
        }
        let mut sum = alloc::vec![0.0f64; dim];
        for j in 0..dim {
            let mut acc = 0.0f64;
            for i in 0..dim {
                acc += self.matrix[i * dim + j] * f64::from(input[i]);
            }
            sum[j] = acc;
        }
        for j in 0..dim {
            output[j] = sum[j] as f32;
        }
        Ok(())
    }

    pub fn verify_orthogonality(&self, tol: f64) -> bool {
        let dim = self.dimension as usize;
        for i in 0..dim {
            for j in 0..dim {
                let mut acc = 0.0f64;
                for k in 0..dim {
                    acc += self.matrix[i * dim + k] * self.matrix[j * dim + k];
                }
                let expected = if i == j { 1.0 } else { 0.0 };
                if (acc - expected).abs() > tol {
                    return false;
                }
            }
        }
        true
    }
}

fn faer_qr_and_correct(data: &[f64], dim: usize) -> Vec<f64> {
    // faer API elided — see docs/design/rust/numerical-semantics.md
    // for the exact sign-correction recipe.
    unimplemented!()
}
```

- [ ] **Step 11: Implement `faer_qr_and_correct`**

Uses `faer::linalg::qr::no_pivoting::compute` to produce `Q` and
`R`, then applies `sign(diag(R))` per column of `Q` to replicate
Python's sign correction.

- [ ] **Step 12: Run rotation tests — expect pass**

```bash
cargo test -p tinyquant-core --test rotation_matrix
```

- [ ] **Step 13: Implement `rotation_cache.rs`**

```rust
use crate::codec::rotation_matrix::RotationMatrix;
use alloc::vec::Vec;
use spin::Mutex;

/// A tiny LRU-ish cache for rotation matrices, keyed by `(seed, dim)`.
///
/// Matches `functools.lru_cache(maxsize=8)` on the Python side.
pub struct RotationCache {
    entries: Mutex<Vec<RotationMatrix>>,
    capacity: usize,
}

impl RotationCache {
    pub const fn new(capacity: usize) -> Self {
        Self { entries: Mutex::new(Vec::new()), capacity }
    }

    pub fn get_or_build(&self, seed: u64, dim: u32) -> RotationMatrix {
        let mut guard = self.entries.lock();
        if let Some(pos) = guard.iter().position(|m| m.seed() == seed && m.dimension() == dim) {
            let found = guard[pos].clone();
            // Move to front (MRU).
            let m = guard.remove(pos);
            guard.insert(0, m);
            return found;
        }
        let built = RotationMatrix::build(seed, dim);
        if guard.len() == self.capacity {
            guard.pop();
        }
        guard.insert(0, built.clone());
        built
    }
}
```

(Requires `spin` workspace dep; add it.)

- [ ] **Step 14: Unit test the cache**

```rust
#[test]
fn cache_returns_identical_matrix_for_repeated_seed() { /* … */ }

#[test]
fn cache_evicts_lru_entry_when_full() { /* … */ }
```

- [ ] **Step 15: Effect-parity test against Python fixture**

Load `fixtures/rotation/seed_42_dim_64.f64.bin`, construct a
`RotationMatrix` via the Rust path, and assert:

1. Orthogonality of both matrices (`verify_orthogonality(1e-10)`).
2. For 1000 deterministic f32 vectors, `max(abs(rust_out - python_out)) < 1e-5`.

The parity test *does not* assert f64 byte equality — see
[[design/rust/numerical-semantics|§R1]] for the rationale.

- [ ] **Step 16: Run full test suite**

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

- [ ] **Step 17: Commit**

```bash
git add rust/crates/tinyquant-core
git commit -m "feat(tinyquant-core): add codec_config and rotation_matrix with parity fixtures"
```

## Acceptance criteria

- `CodecConfig::config_hash()` matches the Python reference for
  every triple in the 120-triple canonical fixture.
- `RotationMatrix::from_config(&cfg)` produces an orthogonal matrix
  for any supported dimension.
- Apply/inverse round trip recovers f32 vectors within `1e-5`
  absolute tolerance.
- Effect-parity against Python rotations for the gold fixture
  `(seed=42, dim=64)` holds within `1e-5`.
- `RotationCache::get_or_build` is deterministic and evicts LRU.
- Full workspace test/clippy/fmt still green.
- No Python-side code modified.

## Out of scope

- Codebook training (phase 14).
- Quantize / dequantize kernels (phase 14).
- Codec service (phase 15).

## See also

- [[plans/rust/phase-12-shared-types-and-errors|Phase 12]]
- [[plans/rust/phase-14-codebook-quantize|Phase 14]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/risks-and-mitigations|Risks and Mitigations]]
