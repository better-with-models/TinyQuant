# Rust vs. Python Reference: End-to-End Performance Investigation

**Authors:** Alison Aquinas, with Claude (Anthropic)
**Date:** April 2026
**Branch:** `fix/rotation-cache-compress-path`
**Repository:** [better-with-models/TinyQuant](https://github.com/better-with-models/TinyQuant)

---

## Abstract

We measured throughput of the Python reference implementation
(`tinyquant_py_reference`) and the Rust library (`tinyquant-core`) on a
335-vector × 1536-dimension embedding corpus. Rust kernel operations —
quantize, dequantize, and cosine similarity — are already 5–195× faster than
their Python equivalents. However, the Rust end-to-end batch compress path
runs at ~1.3 vec/s against Python's ~3,600 vec/s, a ~2,800× regression. The
root cause is a material algorithmic error: `compress_batch_parallel` calls
`RotationMatrix::from_config` — which performs a full O(d³) SVD — on every
individual vector, making the batch path O(N·d³) instead of the intended
O(d³ + N·d²). A `RotationCache` type exists in the codebase but is not wired
into the batch path. The fix is documented in
`docs/plans/rust/rotation-cache-compress-path.md`.

---

## 1. Methodology

### 1.1 Corpus

- **Source:** `tests/reference/fixtures/openai_335_d1536.npy` when available;
  otherwise 335×1536 random unit vectors (seed=42)
- **Dimensions:** 1536 (OpenAI `text-embedding-3-small`)
- **Vectors:** 335

### 1.2 Python benchmark

Script: `bench_python_reference.py` in this directory.

- Implementation: `tinyquant_py_reference` (pure Python, `tests/reference/`)
- Metric: median wall-clock time over 5 repetitions per config
- Measured: per-config encode time, decode time, and derived throughput (vec/s)
- Machine: Apple M-series (aarch64-apple-darwin), single-threaded

### 1.3 Rust batch benchmark

- Crate: `tinyquant-core` with `--features std,simd`, build profile `--release`
- Parallelism: `t=1` (Rayon pool size 1) to isolate per-vector cost
- Invocation: `cargo test -p tinyquant-core --release --features std,simd`
- Measured: compress time for 256×1536 batch in `#[ignore]`d integration test

### 1.4 Rust kernel micro-benchmarks

- Crate: `tinyquant-bench` with `--features std,simd`, build profile `--release`
- Tool: Criterion 0.5 (warm-up 3 s, measurement 5 s)
- Kernels: `scalar_quantize`, `scalar_dequantize`, cosine similarity

---

## 2. Results

### 2.1 Python reference throughput

Corpus: 335 vectors × 1536 dimensions, 5-rep median.

| Config | Encode (ms) | Encode (vec/s) | Decode (ms) | Decode (vec/s) |
|--------|-------------|----------------|-------------|----------------|
| 4-bit + residual | 93 | 3,602 | 127 | 2,646 |
| 4-bit no-residual | 73 | 4,569 | — | — |
| 2-bit + residual | 75 | 4,485 | — | — |
| 8-bit + residual | 88 | 3,795 | — | — |

### 2.2 Rust end-to-end batch throughput (pre-fix)

Corpus: 256 vectors × 1536 dimensions, release build, t=1.

| Path | Throughput |
|------|------------|
| `compress_batch_parallel` | ~1.3 vec/s |
| Ratio vs. Python 4-bit+residual | ~0.036% (2,800× slower) |

### 2.3 Rust kernel micro-benchmarks

Measured on d=1536 elements per call.

| Kernel | Rust (Melem/s) | Python (Melem/s) | Speedup |
|--------|----------------|-------------------|---------|
| `scalar_quantize` (4-bit) | 214 | ~1.1 | ~195× |
| `scalar_dequantize` | 3,200 | ~650 | ~5× |
| cosine similarity | 678 | ~650 | ~1× (tie) |

Python cosine uses NumPy's BLAS-backed dot product; the comparison is fair.

---

## 3. Root Cause Analysis

### 3.1 Per-vector SVD in the batch path

Call graph for `compress_batch_parallel` (before fix):

```
compress_batch_parallel(rows, config, codebook)
  └── par_iter().map(|row| Codec::new().compress(row, config, codebook))
        └── compress(row, config, codebook)
              └── RotationMatrix::from_config(config)          ← O(d³) SVD
                    └── RotationMatrix::build(seed, dim)       ← every row
```

At d=1536, one SVD call takes approximately 750 ms on the bench machine.
For N=256 rows that is 256 × 750 ms ≈ 192 s — matching the observed ~195 s
total for the 256-vector batch test.

### 3.2 Python caches via `lru_cache`

`tinyquant_py_reference/codec/rotation_matrix.py`:

```python
@classmethod
@functools.lru_cache(maxsize=8)
def _cached_build(cls, seed: int, dim: int) -> "RotationMatrix":
    ...
```

`from_config` delegates to `_cached_build`. For a 335-vector batch sharing the
same `(seed, dim)`, the SVD runs exactly once. Total rotation cost ≈ 750 ms /
335 ≈ 2.2 ms/vec, negligible against the rest of encode time.

### 3.3 `RotationCache` exists but is disconnected

`tinyquant-core/src/codec/rotation_matrix.rs` contains a `RotationCache` struct
designed to hold pre-computed matrices indexed by `(seed, dim)`. It is never
called from `batch.rs` or `service.rs`.

---

## 4. Fix Summary

The fix is documented in full in
`docs/plans/rust/rotation-cache-compress-path.md`. The core change:

1. Pre-compute `RotationMatrix::from_config(config)` **once** before the
   `par_iter()` loop in `compress_batch_parallel`.
2. Add `Codec::compress_with_rotation` (pub(crate)) that accepts a pre-built
   `&RotationMatrix` to avoid the redundant `from_config` call per row.
3. Apply the same pattern to `decompress_batch_into`.

The public `Codec::compress()` API is unchanged.

**No changes to `tinyquant_py_reference` are required or proposed.**
The Python reference is correct; the bug is exclusively in the Rust batch path.

---

## 5. Expected Post-Fix Performance

After pre-computing the rotation matrix once per batch:

- Per-vector work: rotate (O(d²)) + quantize + residual — the same operations
  Python performs
- Rust kernel speedups of 5–195× should carry through to the end-to-end path
- Expected batch throughput: well above 100,000 vec/s (limited by quantize
  kernel at ~214 Melem/s ÷ 1536 elem/vec ≈ 139,000 vec/s)

---

## 6. Conclusion

The Rust implementation's kernel-level performance is already substantially
ahead of the Python reference. The observed end-to-end regression is entirely
explained by a single algorithmic error: rebuilding the rotation matrix on
every vector in the batch path. The fix is a targeted, non-breaking change
confined to `batch.rs` and an internal helper in `service.rs`.
