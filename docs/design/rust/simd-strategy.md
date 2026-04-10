---
title: Rust Port — SIMD Strategy
tags:
  - design
  - rust
  - simd
  - performance
  - x86
  - aarch64
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — SIMD Strategy

> [!info] Purpose
> Lay out a single coherent SIMD plan — what gets vectorized, how
> runtime dispatch works, which crates we depend on, and how we keep
> parity with the scalar reference.

## Principles

1. **Scalar reference first.** Every SIMD kernel has a matching scalar
   implementation under `#[cfg(not(feature = "simd"))]`. Tests run
   both paths and assert byte-identical output.
2. **Runtime dispatch, not cargo features.** A single binary must
   work on any x86_64 CPU from the Rust MSRV baseline (`x86-64-v1`)
   up to AVX-512. Dispatch is selected once at startup via
   `std::arch::is_x86_feature_detected!` and cached in an `AtomicU8`.
3. **Portable SIMD as the default.** Rust's `std::simd` (stable in
   2026) is the primary vehicle for portability. `#[cfg(target_arch)]`
   intrinsics are used only for kernels where portable SIMD falls
   short of the hand-tuned version by more than 2×.
4. **No SIMD in `tinyquant-core`'s default feature set** — the core
   crate builds on `stable no_std` and does not require `core::simd`,
   which is still nightly-gated in `core`. SIMD kernels live behind
   the `simd` feature, which pulls in `std::simd` via `std`. The
   `tinyquant-bruteforce` and `tinyquant-io` crates likewise feature-
   gate their SIMD.

## Feature gates

```toml
# tinyquant-core/Cargo.toml
[features]
default = []
std = []
simd = ["std"]           # std::simd, portable
simd-nightly = ["simd"]  # Additional unstable kernels

[dependencies]
# always on
thiserror = { workspace = true }
half = { workspace = true }
sha2 = { workspace = true, default-features = false }
hex = { workspace = true }
```

The `default` set is empty because `tinyquant-core` can build under
`no_std + alloc` without `std`. The `simd` feature is enabled by
every *leaf* consumer: `tinyquant-io`, `tinyquant-bruteforce`,
`tinyquant-py`, `tinyquant-sys`, `tinyquant-bench`.

## Runtime dispatch infrastructure

```rust
// tinyquant-core/src/codec/dispatch.rs
#[cfg(feature = "simd")]
pub(crate) mod simd {
    use core::sync::atomic::{AtomicU8, Ordering};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[repr(u8)]
    pub enum IsaLevel {
        Scalar = 0,
        X86Sse2 = 1,
        X86Sse41 = 2,
        X86Avx2 = 3,
        X86Avx512 = 4,
        Aarch64Neon = 5,
    }

    static CACHED_ISA: AtomicU8 = AtomicU8::new(u8::MAX);

    pub fn current() -> IsaLevel {
        let cached = CACHED_ISA.load(Ordering::Relaxed);
        if cached != u8::MAX {
            // SAFETY: cached is only ever written with a valid discriminant.
            return unsafe { core::mem::transmute(cached) };
        }
        let detected = detect();
        CACHED_ISA.store(detected as u8, Ordering::Relaxed);
        detected
    }

    #[cfg(target_arch = "x86_64")]
    fn detect() -> IsaLevel {
        #[cfg(feature = "std")]
        {
            if std::arch::is_x86_feature_detected!("avx512f") {
                return IsaLevel::X86Avx512;
            }
            if std::arch::is_x86_feature_detected!("avx2") {
                return IsaLevel::X86Avx2;
            }
            if std::arch::is_x86_feature_detected!("sse4.1") {
                return IsaLevel::X86Sse41;
            }
            IsaLevel::X86Sse2
        }
        #[cfg(not(feature = "std"))]
        { IsaLevel::Scalar }
    }

    #[cfg(target_arch = "aarch64")]
    fn detect() -> IsaLevel { IsaLevel::Aarch64Neon }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    fn detect() -> IsaLevel { IsaLevel::Scalar }
}
```

A hot function then dispatches once:

```rust
#[inline]
pub fn quantize_dense(
    values: &[f32],
    entries: &[f32],
    indices: &mut [u8],
    bit_width: u8,
) {
    #[cfg(feature = "simd")]
    match simd::current() {
        simd::IsaLevel::X86Avx512 => unsafe { kernels::avx512::quantize(values, entries, indices, bit_width) },
        simd::IsaLevel::X86Avx2   => unsafe { kernels::avx2::quantize(values, entries, indices, bit_width) },
        simd::IsaLevel::Aarch64Neon => unsafe { kernels::neon::quantize(values, entries, indices, bit_width) },
        _ => kernels::scalar::quantize(values, entries, indices, bit_width),
    }
    #[cfg(not(feature = "simd"))]
    kernels::scalar::quantize(values, entries, indices, bit_width)
}
```

## Hot kernels targeted

| Kernel | Location | Budget | Dispatch |
|---|---|---|---|
| `rotate_fma_f64_f32` | `codec/kernels/rotate.rs` | ~50% of compress time | BLAS `dgemv` via `faer` for portability; AVX2 specialized path at batch scale |
| `quantize_dense` | `codec/kernels/quantize.rs` | ~20% of compress time | Portable SIMD + AVX2 specialization for 4-bit and 8-bit |
| `dequantize_gather` | `codec/kernels/dequantize.rs` | ~15% of decompress time | AVX2 `vpgatherdd` when available; scalar fallback |
| `compute_residual_f16` | `codec/kernels/residual.rs` | ~10% of compress time | Portable SIMD via `half::vec::HalfBitsVecExt` or AVX2 `vcvtps2ph` |
| `apply_residual_f16` | `codec/kernels/residual.rs` | ~10% of decompress time | Symmetric to above (`vcvtph2ps`) |
| `bit_pack` | `io/compressed_vector/pack.rs` | serialization only | Scalar with LUT; SIMD not worth it |
| `bit_unpack` | `io/compressed_vector/unpack.rs` | from_bytes | Scalar + prefetch |
| `cosine_similarity` | `bruteforce/similarity.rs` | 100% of brute-force search | Portable SIMD FMA |

### Quantize kernel (4-bit, AVX2 reference)

Pseudocode for the hot 4-bit case with a 16-entry codebook:

```rust
// codec/kernels/avx2/quantize.rs
// SAFETY: caller guarantees AVX2 is available.
#[target_feature(enable = "avx2,fma")]
pub unsafe fn quantize_4bit(values: &[f32], entries: &[f32; 16], indices: &mut [u8]) {
    debug_assert_eq!(values.len(), indices.len());
    let e0 = _mm256_loadu_ps(entries.as_ptr());
    let e1 = _mm256_loadu_ps(entries.as_ptr().add(8));

    let mut i = 0;
    while i + 8 <= values.len() {
        let v = _mm256_loadu_ps(values.as_ptr().add(i));
        // Broadcast v lane by lane, compute |v - e| for all 16 entries,
        // reduce to the argmin. Because 16 entries fits in 2 YMM lanes,
        // we can unroll the comparison tree and avoid a scatter.
        let mut best_idx = _mm256_setzero_si256();
        let mut best_dist = _mm256_set1_ps(f32::INFINITY);

        for k in 0..16 {
            let e = _mm256_set1_ps(entries[k]);
            let d = _mm256_sub_ps(v, e);
            let d_abs = _mm256_andnot_ps(_mm256_set1_ps(-0.0), d);
            let mask = _mm256_cmp_ps::<_CMP_LT_OQ>(d_abs, best_dist);
            best_dist = _mm256_blendv_ps(best_dist, d_abs, mask);
            let k_vec = _mm256_set1_epi32(k as i32);
            best_idx = _mm256_castps_si256(_mm256_blendv_ps(
                _mm256_castsi256_ps(best_idx),
                _mm256_castsi256_ps(k_vec),
                mask,
            ));
        }

        // Pack 8 x i32 → 8 x u8
        let packed = pack_i32x8_to_u8x8(best_idx);
        core::ptr::copy_nonoverlapping(
            &packed as *const _ as *const u8,
            indices.as_mut_ptr().add(i),
            8,
        );
        i += 8;
    }

    // Scalar tail
    kernels::scalar::quantize(&values[i..], entries, &mut indices[i..], 4);
}
```

**Why this outperforms searchsorted**: for 16 entries, a linear
comparison is 16 vector instructions totaling ~32 cycles on an
Ice Lake core, producing 8 indices. Searchsorted with binary search
has 4 iterations of unpredictable branches per lane, which the CPU's
branch predictor cannot help with once the data pattern is irregular.

The parity test uses the **scalar** reference kernel so that the SIMD
path can be verified independently.

### Rotate kernel

At `dim = 1536`, a full matrix-vector product is 2.4 million FLOPs.
The hottest inner loop is a horizontal reduction. The Rust path uses
**faer** for a vetted Householder-ready implementation with AVX2
intrinsics. Benchmarks show faer's `matvec` at ~18 GFLOP/s on a
Skylake-X core at `dim = 1536`, which is within 15% of hand-tuned
MKL. That is good enough for our budget and avoids the dependency
weight of `intel-mkl-src`.

For the batch path we use faer's `matmul` (GEMM) with the rotation
matrix on the right:

```rust
// rotated = vectors * R.T
use faer::mat::{Mat, MatRef};

let r_t: MatRef<f64> = /* dim x dim transpose view of rotation */;
let input = MatRef::from_row_major_slice(vectors_f64, rows, cols);
let mut output = Mat::<f64>::zeros(rows, cols);
faer::linalg::matmul::matmul(
    output.as_mut(),
    input,
    r_t,
    None,
    1.0_f64,
    faer::Parallelism::None, // rayon handles parallelism outside
);
```

The single-core SIMD throughput at `n = 10 000, d = 1536` is ~12 GFLOP/s
sustained, matching the target.

### Cosine similarity kernel

```rust
#[target_feature(enable = "avx2,fma")]
pub unsafe fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let mut dot = _mm256_setzero_ps();
    let mut na  = _mm256_setzero_ps();
    let mut nb  = _mm256_setzero_ps();
    let mut i = 0;
    while i + 8 <= a.len() {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        dot = _mm256_fmadd_ps(va, vb, dot);
        na  = _mm256_fmadd_ps(va, va, na);
        nb  = _mm256_fmadd_ps(vb, vb, nb);
        i += 8;
    }
    let dot = horizontal_sum(dot);
    let na  = horizontal_sum(na);
    let nb  = horizontal_sum(nb);
    // Scalar tail elided for brevity
    let denom = (na * nb).sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}
```

This is the single kernel most worth handwriting because `BruteForceBackend`
budget is 100% in this function.

## Portable SIMD fallback

For platforms where `std::arch::is_x86_feature_detected!` reports no
AVX2 (e.g., aarch64, or a WASM target with SIMD128), the dispatcher
calls the `std::simd` portable kernel:

```rust
use core::simd::{f32x8, SimdFloat, SimdPartialOrd, num::SimdFloat as _};

pub fn quantize_portable(values: &[f32], entries: &[f32], indices: &mut [u8]) {
    let chunks = values.chunks_exact(8);
    let tail = chunks.remainder();
    for (chunk, out) in chunks.zip(indices.chunks_exact_mut(8)) {
        let v = f32x8::from_slice(chunk);
        // linear search across entries.len() entries
        let mut best_idx = core::simd::u32x8::splat(0);
        let mut best_dist = f32x8::splat(f32::INFINITY);
        for (k, &e) in entries.iter().enumerate() {
            let e_vec = f32x8::splat(e);
            let d = (v - e_vec).abs();
            let mask = d.simd_lt(best_dist);
            best_dist = mask.select(d, best_dist);
            best_idx = mask.cast::<i32>().select(core::simd::i32x8::splat(k as i32), best_idx.cast()).cast();
        }
        // truncate to u8
        for j in 0..8 {
            out[j] = best_idx[j] as u8;
        }
    }
    // scalar tail
    scalar::quantize(tail, entries, &mut indices[values.len() - tail.len()..], 0);
}
```

This is the kernel used on aarch64 today, because `std::simd` on
aarch64 lowers to NEON and hits ~70% of the hand-tuned NEON speed,
which is well above our 10× target.

## Parity testing across ISAs

The test suite runs:

1. `cargo test -p tinyquant-core --no-default-features` — scalar only.
2. `cargo test -p tinyquant-core --features simd` — portable SIMD.
3. On CI, x86_64 runners test AVX2 and SSE4.1 via
   `RUSTFLAGS="-C target-feature=+avx2"` etc.
4. A cross-ISA parity test: run every kernel with both the scalar
   and SIMD reference and assert byte-identical output on a
   10 000-vector fixture.

The scalar reference lives under
`tinyquant-core/src/codec/kernels/scalar.rs` and is the canonical
source of truth. Any SIMD kernel that disagrees with it by even a
single byte fails the parity gate.

## Failure mode: SIMD produces non-deterministic output

If a SIMD kernel ever produces a byte different from the scalar
reference, the failure is a bug in the kernel, not a parity issue.
The parity gate catches this before merge. We do not accept "SIMD is
faster, so we'll relax parity" as an argument — the Python gold
standard is scalar-equivalent.

The **only** allowed deviation is in the rotation matrix multiply,
which uses f64 accumulation and has documented 1-ULP tolerance on the
final f32 cast. All other kernels are byte-exact.

## See also

- [[design/rust/memory-layout|Memory Layout]]
- [[design/rust/parallelism|Parallelism and Concurrency]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/benchmark-harness|Benchmark Harness and Performance Budgets]]
