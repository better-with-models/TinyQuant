---
title: Rust Port — Memory Layout and Allocation Strategy
tags:
  - design
  - rust
  - memory
  - allocation
  - cache
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Memory Layout and Allocation Strategy

> [!info] Purpose
> Pin down every hot data structure's shape, alignment, and
> allocation discipline. Allocator churn is the dominant bottleneck in
> the Python path; the Rust port must avoid it by design, not by
> accident.

## Hot data structures and their shapes

### Row-major batch buffers (canonical representation)

A batch of `n` vectors of dimension `d` is always stored as:

```rust
struct VectorBatch<'a> {
    data: &'a [f32],   // length == n * d, row-major
    rows: usize,       // == n
    cols: usize,       // == d
}
```

The owned variant:

```rust
pub struct OwnedVectorBatch {
    data: Box<[f32]>,  // aligned to 64 bytes (cache line)
    rows: usize,
    cols: usize,
}
```

**Why 64-byte alignment?**

- AVX-512 loads need 64-byte alignment for `vmovaps`.
- x86 cache lines are 64 bytes.
- aarch64 NEON needs only 16-byte alignment but the waste is a few
  bytes per batch and not worth a branch.
- `Box<[f32]>` on the global allocator gives 16-byte alignment by
  default; we force 64-byte alignment via a custom
  `alloc_aligned_f32(len: usize) -> Box<[f32]>` helper in
  `tinyquant-core::codec::buffers`. It uses `alloc::alloc::alloc` with
  `Layout::from_size_align(len * 4, 64)` and the corresponding
  `Layout` passed to `dealloc` via a custom `Drop`.

### CodecConfig layout

```rust
#[repr(C)]
pub struct CodecConfig {
    bit_width: u8,
    _pad: [u8; 3],
    dimension: u32,
    seed: u64,
    residual_enabled: bool,
    _pad2: [u8; 7],
    config_hash: Arc<str>, // 2 * pointer
}
```

Size on 64-bit: 8 (head) + 8 (seed) + 8 (flags+pad) + 16 (Arc) = 40
bytes. `Clone` is one atomic refcount increment plus a memcpy. Used
through `&CodecConfig` in hot paths; never on the stack twice.

### Codebook layout

```rust
pub struct Codebook {
    entries: Arc<[f32]>, // length in {4, 16, 256}
    bit_width: u8,
}
```

Entries are 64-byte aligned (again via the custom allocator) so that
`_mm256_loadu_ps` and `_mm256_load_ps` both hit a cache line.

Because the codebook is tiny (≤ 1 KiB for 8-bit), we always keep it
hot in the L1 cache during a batch pass. Quantization performance is
therefore limited by the searchsorted cost per value.

**Micro-optimization**: for 4-bit and 2-bit codebooks (16 and 4
entries), the linear search is strictly faster than searchsorted and
trivially SIMD-izable. The hot path uses a branchless SIMD linear
search for bit_width ≤ 4 and a searchsorted path for 8-bit. See
[[design/rust/simd-strategy|SIMD Strategy]].

### RotationMatrix layout

```rust
pub struct RotationMatrix {
    matrix: Arc<[f64]>, // row-major, length == dim * dim
    seed: u64,
    dimension: u32,
}
```

64-byte alignment. At `dim = 1536` the matrix is `18 MiB` of f64 data;
it blows past L2 but fits in L3 on most x86 servers. We cannot make it
smaller without breaking parity, so the focus is on **reuse**:

- A single `Arc<[f64]>` is shared by every compression call with the
  same config via the cache (below).
- Batch compression uses BLAS `dgemm` to amortize the cache misses:
  `C[n, d] = B[n, d] @ R.T[d, d]`. At `n = 1024, d = 1536` the arithmetic
  intensity is high enough to saturate L3 bandwidth on a single core
  and stay FLOP-bound.

### CompressedVector layout

```rust
pub struct CompressedVector {
    indices: Arc<[u8]>,        // length == dimension
    residual: Option<Arc<[u8]>>, // length == 2 * dimension when present
    config_hash: Arc<str>,
    dimension: u32,
    bit_width: u8,
}
```

Size on 64-bit: 24 (indices Arc) + 24 (residual Option<Arc>) + 16
(hash Arc) + 8 (dim + bw) = 72 bytes. The heap-side payload for
`dim = 1536, 4-bit + residual` is 1536 bytes (indices, unpacked) + 3072
bytes (residual) = ~5 KiB.

**Why keep indices unpacked in memory?** Because quantization writes
them once and decompression reads them once; packing/unpacking only
happens at serialization boundaries. Keeping them as `u8` avoids a
shift per read on the hot path.

**Alternative considered**: packed-in-memory for storage-bound
workloads (`4-bit * 1536 = 768 bytes` vs 1536 unpacked). Rejected
because the unpacking cost on decompression dominates the hot path:
profiling the Python path shows that `from_bytes` is 40% of the total
when decompressing a million vectors. Keeping memory representations
unpacked and only packing at the byte boundary is a net win.

### Corpus layout

```rust
pub struct Corpus {
    corpus_id: CorpusId,
    codec_config: CodecConfig,
    codebook: Codebook,
    compression_policy: CompressionPolicy,
    // Insertion-ordered map so that pending events list is stable.
    vectors: indexmap::IndexMap<VectorId, VectorEntry, ahash::RandomState>,
    metadata: BTreeMap<String, serde_json::Value>,
    pending_events: Vec<CorpusEvent>,
}
```

`indexmap::IndexMap` is used instead of `HashMap` because:

1. Python's `dict` preserves insertion order; tests rely on it.
2. Iteration over vectors during `decompress_all` needs a stable order
   for byte-parity with Python's output.

`ahash::RandomState` gives a 2-3× speedup over `RandomState` for
small string keys without materially weakening hash security; since
corpus IDs and vector IDs are trusted internal identifiers, this is a
reasonable trade-off.

## Rotation matrix cache

Python uses `functools.lru_cache(maxsize=8)` keyed by
`(seed, dimension)`. Rust replicates this with a cross-thread safe
variant:

```rust
// tinyquant-core/src/codec/rotation_cache.rs
use alloc::sync::Arc;
use spin::Mutex;

pub(crate) struct RotationCache {
    entries: Mutex<SmallRingBuffer<RotationMatrix, 8>>,
}

impl RotationCache {
    pub fn get_or_build(&self, seed: u64, dim: u32) -> RotationMatrix { /* … */ }
}
```

Properties:

- `no_std`-compatible (uses `spin::Mutex`, not `std::sync::Mutex`).
- Ring buffer instead of a proper LRU because the access pattern in
  practice is one hot config, so a 1-entry cache would work; 8 gives
  headroom for multi-config workloads.
- Lock-free reads for the hot case via a relaxed-atomics fast path
  when the first entry matches (deferred to phase-13 optimization).
- One global `RotationCache` per process, behind `lazy_static!` in a
  `std`-only init function exposed from `tinyquant-io`. The core crate
  exposes only the type; applications instantiate and pass in the
  cache.

Alternative considered: per-call cache parameter. Rejected because
the Python API is implicit and downstream migrations would break.

## Batch compression allocation discipline

The hot `compress_batch` function allocates exactly these things
once per call (not per vector):

1. One `Box<[u8]>` of length `rows * cols` for the unpacked indices.
2. One `Box<[u8]>` of length `2 * rows * cols` for the residuals (if enabled).
3. One `Box<[f32]>` of length `rows * cols` for the rotated vectors.
4. One `Vec<CompressedVector>` of length `rows` (the return value).

Inside the hot loop, per-vector allocations are **forbidden**. A
clippy lint (`clippy::alloc_instead_of_core`) plus a custom xtask
check that greps for `Vec::with_capacity`/`Box::new` inside hot
functions enforces this. The return-value `CompressedVector`s own
slices of the shared buffers via `Arc::from(subslice_to_box)`.

Wait — a slice of a `Box<[u8]>` cannot be converted to an `Arc<[u8]>`
without copying. The design choice here is:

**Option A**: each returned `CompressedVector` gets its own
`Arc::from(sub_box)`, which does allocate per vector. Still O(n)
allocations per batch.

**Option B**: return a `BatchedCompressedVectors` struct that stores
the `rows * cols` indices buffer plus a length table, and exposes
`get(i) -> CompressedVectorView<'_>` zero-copy. Individual owned
`CompressedVector`s are produced lazily.

We ship **both**: `compress_batch` allocates per-vector for API
parity with Python (`list[CompressedVector]`), and a new method
`compress_batch_packed` returns the `BatchedCompressedVectors` pool
for zero-alloc consumers. The parity gate uses the former; the
benchmark targets use the latter for the stretch numbers.

## Residual buffer discipline

The residual is `2 * dimension` bytes per vector and dominates the
memory footprint when enabled. The batch path allocates one giant
buffer and hands out slices:

```rust
pub struct ResidualPool {
    data: Box<[u8]>,           // 2 * rows * cols
    stride: usize,             // 2 * cols
}

impl ResidualPool {
    pub fn slice(&self, i: usize) -> &[u8] {
        let start = i * self.stride;
        &self.data[start..start + self.stride]
    }
}
```

For the owned `CompressedVector` return path, we `Arc::from(copy)`
each slice. For the batched view path, consumers borrow from the pool.

## Zero-copy deserialization (mmap path)

A memory-mapped corpus file contains a sequence of serialized
`CompressedVector`s, each prefixed with its byte length:

```
[MAGIC "TQCV" (4 bytes)]
[VERSION u16 LE]
[COUNT u64 LE]
[FLAGS u16 LE]        (bit 0 = residual enabled throughout file)
[RESERVED 16 bytes]
[--- per-vector ---]
[LEN u32 LE]
[PAYLOAD ...]         (exactly Python-format CompressedVector bytes)
```

The `CompressedVectorView<'a>` type (`tinyquant-io::zero_copy::view`)
holds `&'a [u8]` slices into the mmap:

```rust
pub struct CompressedVectorView<'a> {
    version: u8,
    config_hash: &'a str,
    dimension: u32,
    bit_width: u8,
    packed_indices: &'a [u8],
    residual: Option<&'a [u8]>,
}

impl<'a> CompressedVectorView<'a> {
    pub fn iter_indices(&self) -> IndicesIter<'a> { /* … */ }

    /// Copy into a pre-allocated buffer (zero alloc).
    pub fn unpack_into(&self, out: &mut [u8]) -> Result<(), IoError> { /* … */ }

    /// Returns an owned CompressedVector by allocating exactly twice.
    pub fn to_owned(&self) -> CompressedVector { /* … */ }
}
```

A corpus iterator yields views without touching the allocator:

```rust
pub struct CorpusFileIter<'a> {
    remaining: &'a [u8],
    count: u64,
}

impl<'a> Iterator for CorpusFileIter<'a> {
    type Item = Result<CompressedVectorView<'a>, IoError>;
}
```

The entire streaming-decompress path (`tinyquant-bench
codec_decompress_batch`) runs through this iterator and never
allocates inside the inner loop.

## Allocator choice

- **Default**: the system allocator. No dependency weight.
- **Feature `jemalloc`**: enable `jemallocator` in `tinyquant-py` and
  `tinyquant-sys` for production workloads on Linux. Benchmarks show
  a 10-15% improvement on batch paths because the Box<[u8]> churn on
  the return path is smaller.
- **Feature `mimalloc`**: as a Windows alternative.
- `tinyquant-core` never chooses an allocator; it is `no_std` and
  relies on `alloc`'s global.

## Thread safety

All types in `tinyquant-core` are `Send + Sync` by construction:

- `CodecConfig`, `Codebook`, `RotationMatrix`, `CompressedVector`
  contain `Arc<[T]>` and POD fields → automatically `Send + Sync`.
- `Codec` is a ZST → trivially `Send + Sync`.
- `Corpus` is `Send` but not `Sync` (it has `&mut self` mutators).
  Callers can wrap it in a `Mutex` if they need cross-thread mutation.
- `RotationCache` is `Send + Sync` via the `spin::Mutex`.

A compile-time check asserts this:

```rust
// tinyquant-core/tests/thread_safety.rs
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn types_are_send_sync() {
    assert_send_sync::<CodecConfig>();
    assert_send_sync::<Codebook>();
    assert_send_sync::<RotationMatrix>();
    assert_send_sync::<CompressedVector>();
    assert_send_sync::<Codec>();
}
```

## Budget table (bytes per dim = 1536, batch of 10 000)

| Buffer | Owned bytes | Notes |
|---|---|---|
| Input vectors (f32) | 60 MiB | Caller-provided; not allocated by codec |
| Rotated vectors (f32) | 60 MiB | Allocated once per batch call |
| Reconstructed stage-1 (f32) | 60 MiB | Reused the rotated buffer |
| Codebook | 1 KiB | Shared across entire process via Arc |
| Rotation matrix (f64) | 18 MiB | Shared across entire process via Arc |
| Indices (u8) | 15 MiB | Allocated once |
| Residuals (f16 bytes) | 30 MiB | Allocated once |
| Return Vec<CompressedVector> | ~460 KiB headers + above slices | Per-vector Arc overhead |
| **Total peak** | ~245 MiB | Fits comfortably on an 8 GiB container |

With `compress_batch_packed` the return-path overhead drops to ~30 KiB
(just the length table), so the zero-alloc path is ~215 MiB peak.

## See also

- [[design/rust/simd-strategy|SIMD Strategy]]
- [[design/rust/parallelism|Parallelism and Concurrency]]
- [[design/rust/serialization-format|Serialization Format]]
- [[design/rust/benchmark-harness|Benchmark Harness and Performance Budgets]]
