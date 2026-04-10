---
title: Rust Port — Type Mapping from Python
tags:
  - design
  - rust
  - types
  - parity
  - api
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Type Mapping from Python

> [!info] Purpose
> One-to-one mapping of every public Python symbol to its Rust
> counterpart. This document is the reference that all tests, the C
> ABI, and the pyo3 bindings must agree with. Any behavior drift from
> the Python gold standard is a bug until the behavior layer is
> explicitly updated.

> [!note] Naming conventions
> Python `CodecConfig` becomes Rust `CodecConfig` (PascalCase
> preserved). Python methods `compress`/`decompress` become Rust methods
> `compress`/`decompress` (snake_case already). Free functions like
> `tinyquant_cpu.codec.compress` become associated functions on a zero
> sized `Codec` service type, *and* are re-exported as free functions
> `tinyquant_core::codec::compress` for ergonomic parity.

## Shared type aliases

Python (`src/tinyquant_cpu/_types.py`):

```python
VectorId = str
ConfigHash = str
CorpusId = str
Vector = NDArray[np.float32]
VectorBatch = NDArray[np.float32]
```

Rust (`tinyquant-core/src/types.rs`):

```rust
//! Shared type aliases mirroring `tinyquant_cpu._types`.

use alloc::string::String;
use alloc::sync::Arc;

/// Stable identity for a vector inside a [`Corpus`].
pub type VectorId = Arc<str>;

/// SHA-256 hex digest identifying a `CodecConfig`.
pub type ConfigHash = Arc<str>;

/// Stable identity for a [`Corpus`].
pub type CorpusId = Arc<str>;

/// Owned 1-D FP32 vector.
pub type Vector = alloc::vec::Vec<f32>;

/// Borrowed 1-D FP32 vector (immutable view).
pub type VectorSlice<'a> = &'a [f32];
```

Rationale: `Arc<str>` lets identities be cloned cheaply across events
and entries without allocating; the Python `str` type has the same
shareability property. `Vector` stays an owned `Vec<f32>` because NumPy
arrays also own their buffer after `CompressedVector` freezes them.
Functions that need a batch accept `&[f32]` plus a `dim: usize` and
interpret it row-major.

## Codec layer

### CodecConfig

**Python** (`src/tinyquant_cpu/codec/codec_config.py`):

```python
@dataclass(frozen=True)
class CodecConfig:
    bit_width: int
    seed: int
    dimension: int
    residual_enabled: bool = True

    # num_codebook_entries: int (property)
    # config_hash: ConfigHash (property, SHA-256 hex)
```

**Rust** (`tinyquant-core/src/codec/codec_config.rs`):

```rust
use crate::errors::CodecError;
use crate::types::ConfigHash;
use core::hash::{Hash, Hasher};

/// Supported quantization widths (must stay in sync with Python).
pub const SUPPORTED_BIT_WIDTHS: &[u8] = &[2, 4, 8];

/// Immutable configuration snapshot that fully determines codec behavior.
///
/// Equivalent to `tinyquant_cpu.codec.codec_config.CodecConfig`.
#[derive(Clone, Debug, Eq)]
pub struct CodecConfig {
    bit_width: u8,
    seed: u64,
    dimension: u32,
    residual_enabled: bool,
    // Config hash is computed lazily and cached behind an Arc<str>
    // so that clones don't re-hash. Lazy init without OnceCell because
    // tinyquant-core is no_std; we compute on first access and store
    // the result under &mut access via interior mutability? No — we
    // compute eagerly at construction. See "invariants" below.
    config_hash: ConfigHash,
}

impl CodecConfig {
    /// Construct a new config, validating all invariants.
    ///
    /// # Errors
    ///
    /// Returns `CodecError::UnsupportedBitWidth` if `bit_width`
    /// is not one of `{2, 4, 8}`, `CodecError::InvalidDimension`
    /// if `dimension` is zero, and the seed invariant is preserved
    /// via the `u64` type (no negative seeds representable).
    pub fn new(
        bit_width: u8,
        seed: u64,
        dimension: u32,
        residual_enabled: bool,
    ) -> Result<Self, CodecError> {
        if !SUPPORTED_BIT_WIDTHS.contains(&bit_width) {
            return Err(CodecError::UnsupportedBitWidth { got: bit_width });
        }
        if dimension == 0 {
            return Err(CodecError::InvalidDimension { got: 0 });
        }
        let config_hash = Self::compute_hash(bit_width, seed, dimension, residual_enabled);
        Ok(Self {
            bit_width,
            seed,
            dimension,
            residual_enabled,
            config_hash,
        })
    }

    /// Bit width per quantized coordinate (2, 4, or 8).
    #[inline]
    pub const fn bit_width(&self) -> u8 { self.bit_width }

    /// Deterministic seed for rotation matrix generation.
    #[inline]
    pub const fn seed(&self) -> u64 { self.seed }

    /// Expected input vector dimensionality.
    #[inline]
    pub const fn dimension(&self) -> u32 { self.dimension }

    /// Whether stage-2 residual correction is enabled.
    #[inline]
    pub const fn residual_enabled(&self) -> bool { self.residual_enabled }

    /// Number of quantization levels: `2u32.pow(bit_width as u32)`.
    #[inline]
    pub const fn num_codebook_entries(&self) -> u32 {
        1u32 << self.bit_width
    }

    /// SHA-256 hex digest of all fields (matches Python byte for byte).
    #[inline]
    pub fn config_hash(&self) -> &ConfigHash { &self.config_hash }

    /// Compute the hash in the exact canonical form Python uses:
    ///
    /// `"CodecConfig(bit_width={b},seed={s},dimension={d},residual_enabled={r})"`
    ///
    /// with `r` as `True` or `False` (Python's `str(bool)`).
    fn compute_hash(
        bit_width: u8,
        seed: u64,
        dimension: u32,
        residual_enabled: bool,
    ) -> ConfigHash {
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
        let digest = hasher.finalize();
        Arc::from(hex::encode(digest))
    }
}

impl PartialEq for CodecConfig {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.bit_width == other.bit_width
            && self.seed == other.seed
            && self.dimension == other.dimension
            && self.residual_enabled == other.residual_enabled
    }
}

impl Hash for CodecConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bit_width.hash(state);
        self.seed.hash(state);
        self.dimension.hash(state);
        self.residual_enabled.hash(state);
    }
}
```

**Parity invariants**

1. `CodecConfig::config_hash()` must equal
   `tinyquant_cpu.codec.CodecConfig.config_hash` **as a string** for
   every valid triple. A parity test in `tinyquant-py/tests/test_parity.py`
   enumerates 64 combinations and asserts equality.
2. `bit_width` is `u8`, not `u32`, because the supported set only has
   three values. The pyo3 binding accepts Python `int` and errors on
   values outside `u8::MAX` before delegating.
3. `seed` is `u64`. Python `int` values may exceed `u64::MAX`; pyo3
   binding converts with `TryFrom<&PyInt>` and errors otherwise.
4. Equality and hashing ignore the cached `config_hash` field.

### Codebook

**Python** — frozen dataclass with `entries: NDArray[f32]` and
`bit_width: int`, a `train(vectors, config)` classmethod (uniform
quantiles), and `quantize` / `dequantize` methods.

**Rust** (`tinyquant-core/src/codec/codebook.rs`):

```rust
use crate::codec::codec_config::CodecConfig;
use crate::errors::CodecError;
use alloc::boxed::Box;
use alloc::sync::Arc;

/// Immutable quantization lookup table.
///
/// `entries` is sorted ascending with exactly `2.pow(bit_width)`
/// distinct values. The buffer is heap-allocated and shared through
/// `Arc` so that clones are O(1).
#[derive(Clone, Debug)]
pub struct Codebook {
    entries: Arc<[f32]>,
    bit_width: u8,
}

impl Codebook {
    /// Build a codebook from an already-sorted entry slice.
    ///
    /// # Errors
    ///
    /// `CodecError::CodebookEntryCount` if `entries.len() != 2^bit_width`.
    /// `CodecError::CodebookNotSorted` if entries are not non-decreasing.
    /// `CodecError::CodebookDuplicate` if any two adjacent entries are
    /// byte-equal (matches Python `np.unique` guard).
    pub fn new(entries: Box<[f32]>, bit_width: u8) -> Result<Self, CodecError> { /* … */ }

    /// Train from representative vectors using uniform quantile
    /// estimation, matching `numpy.quantile(flat, linspace(0, 1, k))`.
    ///
    /// # Errors
    ///
    /// `CodecError::InsufficientTrainingData` if there are fewer
    /// distinct values than `2^bit_width`.
    pub fn train(
        vectors: &[f32],
        rows: usize,
        cols: usize,
        config: &CodecConfig,
    ) -> Result<Self, CodecError> { /* … */ }

    #[inline]
    pub fn entries(&self) -> &[f32] { &self.entries }

    #[inline]
    pub const fn bit_width(&self) -> u8 { self.bit_width }

    #[inline]
    pub fn num_entries(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Map continuous values to nearest codebook indices, writing into
    /// a pre-allocated output buffer. Python's `quantize` returns a new
    /// array; Rust avoids the allocation by taking `&mut [u8]`.
    ///
    /// # Errors
    ///
    /// `CodecError::LengthMismatch` if `values.len() != indices.len()`.
    pub fn quantize_into(
        &self,
        values: &[f32],
        indices: &mut [u8],
    ) -> Result<(), CodecError> { /* … */ }

    /// Dequantize indices back to FP32 values.
    ///
    /// # Errors
    ///
    /// `CodecError::IndexOutOfRange` if any `indices[i] >= num_entries`.
    pub fn dequantize_into(
        &self,
        indices: &[u8],
        values: &mut [f32],
    ) -> Result<(), CodecError> { /* … */ }
}
```

**Numerical parity notes**

1. Python's training uses `np.quantile(flat.astype(float64),
   np.linspace(0, 1, k)).astype(float32)` then sorts. Rust replicates
   this exactly: promote to f64, compute the linspace, use `faer`'s
   quantile (or a hand-written equivalent of NumPy's
   linear-interpolation quantile which matches NumPy's default
   `method="linear"`), cast to f32, sort ascending. A parity test
   generates 1 000 random seeds and compares byte-for-byte against
   Python.
2. `quantize_into` uses `searchsorted` + nearest-neighbor
   comparison. The Rust version implements branchless nearest-neighbor
   using `simd::cmp::SimdPartialOrd` on wide lanes.

### RotationMatrix

**Python** — stores a `float64` `(dim, dim)` matrix, generated from
`np.random.default_rng(seed).standard_normal((d, d))` then
`np.linalg.qr(...)` with a sign correction via `np.diag(r)`.

**Rust** (`tinyquant-core/src/codec/rotation_matrix.rs`):

```rust
use crate::codec::codec_config::CodecConfig;
use crate::errors::CodecError;
use alloc::sync::Arc;

/// Deterministically generated orthogonal matrix, `dim x dim` in f64.
///
/// Clones share the underlying matrix via `Arc`.
#[derive(Clone, Debug)]
pub struct RotationMatrix {
    /// Row-major f64 matrix of length `dim * dim`.
    matrix: Arc<[f64]>,
    seed: u64,
    dimension: u32,
}

impl RotationMatrix {
    /// Build (or fetch from cache) the rotation matrix for a config.
    pub fn from_config(config: &CodecConfig) -> Self { /* … */ }

    /// Build without going through the shared cache.
    pub fn build(seed: u64, dimension: u32) -> Self { /* … */ }

    #[inline]
    pub fn matrix(&self) -> &[f64] { &self.matrix }

    #[inline]
    pub const fn dimension(&self) -> u32 { self.dimension }

    /// Rotate `input` into `output`. Requires
    /// `input.len() == output.len() == self.dimension as usize`.
    ///
    /// Internally promotes to f64, multiplies, casts back to f32.
    pub fn apply_into(&self, input: &[f32], output: &mut [f32]) -> Result<(), CodecError> { /* … */ }

    /// Apply `matrix.transpose()` (valid because orthogonal).
    pub fn apply_inverse_into(
        &self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<(), CodecError> { /* … */ }

    /// `||R R^T - I||_{inf} < tol`.
    pub fn verify_orthogonality(&self, tol: f64) -> bool { /* … */ }
}
```

**Determinism recipe** (see
[[design/rust/numerical-semantics|Numerical Semantics]] for the full
justification):

1. Initialize a `ChaCha20Rng` with the 64-bit seed in the same byte
   layout NumPy uses for its `default_rng(seed)` BitGenerator. For
   parity on the hot path, we take a different approach: we generate
   the same standard-normal samples by cross-checking against a
   pre-baked fixture captured from Python. If NumPy's BitGenerator
   stream cannot be reproduced in pure Rust (it uses PCG64 + Ziggurat),
   the design defaults to **the pre-baked fixture approach** — the
   rotation matrix is generated in Rust via `faer`'s QR of a random
   matrix seeded the same way, and parity is asserted on the *effect*
   (post-rotation vectors match to 1e-5) rather than on the raw matrix
   bytes. See [[design/rust/numerical-semantics|Numerical Semantics]]
   for the full analysis and the fallback plan.
2. QR decomposition via `faer`'s `faer::linalg::qr::no_pivoting`.
3. Sign correction: `q[:, j] *= sign(r[j, j])`.

### CompressedVector

**Python** — frozen dataclass with `indices: NDArray[u8]`,
`residual: Optional[bytes]`, `config_hash: str`, `dimension: int`,
`bit_width: int`, plus `to_bytes`/`from_bytes`.

**Rust core** (`tinyquant-core/src/codec/compressed_vector.rs`):

```rust
use crate::types::ConfigHash;
use alloc::sync::Arc;
use alloc::boxed::Box;

/// Output of the codec compression pipeline.
///
/// Carries *unpacked* u8 indices and an optional residual byte blob.
/// Serialization lives in `tinyquant-io::compressed_vector`.
#[derive(Clone, Debug)]
pub struct CompressedVector {
    /// One u8 per dimension. Upper bits of each byte are zero when
    /// `bit_width < 8`. Length is exactly `dimension`.
    indices: Arc<[u8]>,
    /// `None` if residual was disabled. When present, always 2 bytes
    /// per dimension (f16 little-endian).
    residual: Option<Arc<[u8]>>,
    config_hash: ConfigHash,
    dimension: u32,
    bit_width: u8,
}

impl CompressedVector {
    /// Validated constructor matching Python `__post_init__`.
    pub fn new(
        indices: Box<[u8]>,
        residual: Option<Box<[u8]>>,
        config_hash: ConfigHash,
        dimension: u32,
        bit_width: u8,
    ) -> Result<Self, CodecError> { /* … */ }

    #[inline]
    pub fn indices(&self) -> &[u8] { &self.indices }

    #[inline]
    pub fn residual(&self) -> Option<&[u8]> { self.residual.as_deref() }

    #[inline]
    pub fn config_hash(&self) -> &ConfigHash { &self.config_hash }

    #[inline]
    pub const fn dimension(&self) -> u32 { self.dimension }

    #[inline]
    pub const fn bit_width(&self) -> u8 { self.bit_width }

    #[inline]
    pub fn has_residual(&self) -> bool { self.residual.is_some() }

    /// Matches Python `size_bytes`: packed footprint + residual length.
    pub fn size_bytes(&self) -> usize {
        let packed = ((self.dimension as usize) * (self.bit_width as usize) + 7) / 8;
        packed + self.residual.as_ref().map_or(0, |r| r.len())
    }
}
```

The **serialization** (`to_bytes` / `from_bytes`) lives in the
`tinyquant-io` crate as free functions so that `tinyquant-core` stays
`no_std` and IO-free:

```rust
// tinyquant-io/src/compressed_vector/to_bytes.rs
pub fn to_bytes(cv: &CompressedVector) -> Vec<u8> { /* … */ }

// tinyquant-io/src/compressed_vector/from_bytes.rs
pub fn from_bytes(data: &[u8]) -> Result<CompressedVector, IoError> { /* … */ }
```

This keeps the core leaf crate pure.

### Codec service

**Python** — stateless class with `compress`, `decompress`,
`compress_batch`, `decompress_batch`, `build_codebook`, `build_rotation`.

**Rust** (`tinyquant-core/src/codec/codec.rs`):

```rust
use crate::codec::*;
use crate::errors::CodecError;

/// Zero-sized stateless service — equivalent to `tinyquant_cpu.codec.Codec`.
#[derive(Default, Debug, Clone, Copy)]
pub struct Codec;

impl Codec {
    pub const fn new() -> Self { Self }

    /// Compress a single vector; matches Python exactly.
    pub fn compress(
        &self,
        vector: &[f32],
        config: &CodecConfig,
        codebook: &Codebook,
    ) -> Result<CompressedVector, CodecError> { /* … */ }

    pub fn decompress_into(
        &self,
        compressed: &CompressedVector,
        config: &CodecConfig,
        codebook: &Codebook,
        output: &mut [f32],
    ) -> Result<(), CodecError> { /* … */ }

    /// Allocating variant to match Python's `decompress` return type.
    pub fn decompress(
        &self,
        compressed: &CompressedVector,
        config: &CodecConfig,
        codebook: &Codebook,
    ) -> Result<Vec<f32>, CodecError> { /* … */ }

    /// Row-major batch compress. `vectors.len() == rows * cols`.
    pub fn compress_batch(
        &self,
        vectors: &[f32],
        rows: usize,
        cols: usize,
        config: &CodecConfig,
        codebook: &Codebook,
    ) -> Result<Vec<CompressedVector>, CodecError> { /* … */ }

    /// Same for decompression. Writes into `output` (row-major).
    pub fn decompress_batch_into(
        &self,
        compressed: &[CompressedVector],
        config: &CodecConfig,
        codebook: &Codebook,
        output: &mut [f32],
    ) -> Result<(), CodecError> { /* … */ }

    pub fn build_codebook(
        &self,
        training_vectors: &[f32],
        rows: usize,
        cols: usize,
        config: &CodecConfig,
    ) -> Result<Codebook, CodecError> { /* … */ }

    pub fn build_rotation(&self, config: &CodecConfig) -> RotationMatrix { /* … */ }
}

/// Module-level convenience wrappers (match Python
/// `tinyquant_cpu.codec.compress` / `decompress`).
pub fn compress(
    vector: &[f32],
    config: &CodecConfig,
    codebook: &Codebook,
) -> Result<CompressedVector, CodecError> {
    Codec.compress(vector, config, codebook)
}

pub fn decompress(
    compressed: &CompressedVector,
    config: &CodecConfig,
    codebook: &Codebook,
) -> Result<Vec<f32>, CodecError> {
    Codec.decompress(compressed, config, codebook)
}
```

## Corpus layer

### CompressionPolicy

```rust
// tinyquant-core/src/corpus/compression_policy.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CompressionPolicy {
    /// Full codec pipeline.
    Compress,
    /// Store FP32 verbatim.
    Passthrough,
    /// Downcast to FP16.
    Fp16,
}

impl CompressionPolicy {
    pub const fn requires_codec(self) -> bool {
        matches!(self, Self::Compress)
    }

    /// Matches Python `storage_dtype()` semantically (returns a tag
    /// rather than a NumPy dtype).
    pub const fn storage_tag(self) -> StorageTag {
        match self {
            Self::Compress => StorageTag::U8,
            Self::Passthrough => StorageTag::F32,
            Self::Fp16 => StorageTag::F16,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum StorageTag { U8, F16, F32 }
```

pyo3 layer exposes these as `CompressionPolicy.COMPRESS`,
`PASSTHROUGH`, `FP16` with string values matching Python.

### VectorEntry, Corpus, Events

Full signatures are inlined in
[[design/rust/numerical-semantics|Numerical Semantics]] and
[[design/rust/serialization-format|Serialization Format]] — the
important design points:

- `VectorEntry` has identity equality by `vector_id` only (mirrors
  Python `__eq__`); `derive(PartialEq)` is *not* used, we hand-write it.
- `Corpus` is the sole mutable aggregate. All mutating methods take
  `&mut self` and return an event that the caller drains via
  `pending_events(&mut self) -> Vec<CorpusEvent>`.
- Events are an `enum CorpusEvent` tagged union, not separate types,
  so pattern matching is ergonomic. Each variant carries the same
  fields Python's dataclass events carry.

```rust
#[derive(Clone, Debug)]
pub enum CorpusEvent {
    Created {
        corpus_id: CorpusId,
        codec_config: CodecConfig,
        compression_policy: CompressionPolicy,
        timestamp: Timestamp,
    },
    VectorsInserted {
        corpus_id: CorpusId,
        vector_ids: Arc<[VectorId]>,
        count: u32,
        timestamp: Timestamp,
    },
    Decompressed {
        corpus_id: CorpusId,
        vector_count: u32,
        timestamp: Timestamp,
    },
    PolicyViolationDetected {
        corpus_id: CorpusId,
        kind: ViolationKind,
        detail: Arc<str>,
        timestamp: Timestamp,
    },
}
```

`Timestamp` is a plain `i64` nanoseconds-since-epoch in the core crate
(no `std::time`); `tinyquant-io` and `tinyquant-py` convert to
`SystemTime` / `datetime.datetime` at the boundary. This keeps the
core `no_std`.

## Backend layer

### SearchBackend trait

```rust
// tinyquant-core/src/backend/protocol.rs
use crate::errors::BackendError;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub vector_id: VectorId,
    pub score: f32,
}

// Descending order by score (Python __lt__ returns self.score > other.score).
impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        other.score.partial_cmp(&self.score)
    }
}

pub trait SearchBackend {
    fn ingest(
        &mut self,
        vectors: &[(VectorId, Vec<f32>)],
    ) -> Result<(), BackendError>;

    fn search(
        &self,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>, BackendError>;

    fn remove(&mut self, vector_ids: &[VectorId]) -> Result<(), BackendError>;
}
```

`dyn SearchBackend` is object-safe. Generic parameters are reserved
for performance-critical implementations (`BruteForceBackend<'a>`);
the trait itself is dyn-safe for ergonomic use at the corpus boundary.

### BruteForceBackend

Lives in `tinyquant-bruteforce`. Owns a `Vec<(VectorId, Vec<f32>)>`
plus a cached `dim`. Search computes cosine similarity per vector,
writes `(score, id_index)` pairs into a bounded heap of size `top_k`.

### PgvectorAdapter

Lives in `tinyquant-pgvector`. Takes a connection factory closure
(`impl Fn() -> postgres::Client + Send + Sync`) to mirror Python's
`connection_factory: Callable[[], Any]`.

## Error taxonomy (full enum)

```rust
// tinyquant-core/src/errors.rs
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodecError {
    #[error("bit_width must be one of [2, 4, 8], got {got}")]
    UnsupportedBitWidth { got: u8 },

    #[error("dimension must be > 0, got {got}")]
    InvalidDimension { got: u32 },

    #[error("vector length {got} does not match config dimension {expected}")]
    DimensionMismatch { expected: u32, got: u32 },

    #[error("codebook bit_width {got} does not match config bit_width {expected}")]
    CodebookIncompatible { expected: u8, got: u8 },

    #[error("compressed config_hash {got:?} does not match config hash {expected:?}")]
    ConfigMismatch { expected: Arc<str>, got: Arc<str> },

    #[error("codebook must have {expected} entries for bit_width={bit_width}, got {got}")]
    CodebookEntryCount { expected: u32, got: u32, bit_width: u8 },

    #[error("codebook entries must be sorted ascending")]
    CodebookNotSorted,

    #[error("codebook must contain {expected} distinct values, got {got}")]
    CodebookDuplicate { expected: u32, got: u32 },

    #[error("insufficient training data for {expected} distinct entries")]
    InsufficientTrainingData { expected: u32 },

    #[error("index {index} is out of range [0, {bound})")]
    IndexOutOfRange { index: u8, bound: u32 },

    #[error("input and output lengths disagree: {left} vs {right}")]
    LengthMismatch { left: usize, right: usize },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CorpusError {
    #[error("dimension mismatch: {source}")]
    DimensionMismatch { #[from] source: CodecError },

    #[error("duplicate vector_id {id:?}")]
    DuplicateVectorId { id: VectorId },

    #[error("unknown vector_id {id:?}")]
    UnknownVectorId { id: VectorId },

    #[error("codec error: {0}")]
    Codec(#[from] CodecError),
}

#[derive(Debug, Error, Clone)]
pub enum BackendError {
    #[error("backend is empty")]
    Empty,

    #[error("top_k must be > 0")]
    InvalidTopK,

    #[error("adapter error: {0}")]
    Adapter(Arc<str>),
}
```

**Exception mapping in `tinyquant-py`**:

| Rust error variant | Python exception |
|---|---|
| `CodecError::DimensionMismatch { .. }` | `DimensionMismatchError` |
| `CodecError::ConfigMismatch { .. }` | `ConfigMismatchError` |
| `CodecError::CodebookIncompatible { .. }` | `CodebookIncompatibleError` |
| `CorpusError::DuplicateVectorId { .. }` | `DuplicateVectorError` |
| Everything else | `ValueError` (matching Python base class) |

## Public re-export surface

`tinyquant-core/src/lib.rs`:

```rust
pub mod codec;
pub mod corpus;
pub mod backend;
pub mod errors;
pub mod types;

pub mod prelude {
    pub use crate::codec::{
        Codebook, Codec, CodecConfig, CompressedVector, RotationMatrix,
        SUPPORTED_BIT_WIDTHS, compress, decompress,
    };
    pub use crate::corpus::{
        CompressionPolicy, Corpus, CorpusEvent, VectorEntry, ViolationKind,
    };
    pub use crate::backend::{SearchBackend, SearchResult};
    pub use crate::errors::{BackendError, CodecError, CorpusError};
    pub use crate::types::{ConfigHash, CorpusId, Vector, VectorId, VectorSlice};
}
```

This mirrors the top-level Python `__init__.py` re-exports exactly.

## See also

- [[design/rust/crate-topology|Crate Topology]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/error-model|Error Model]]
- [[design/rust/serialization-format|Serialization Format]]
- [[design/rust/ffi-and-bindings|FFI and Bindings]]
