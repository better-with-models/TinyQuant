# TinyQuant: CPU-Only Vector Quantization Library

**Date:** 2026-04-08
**Status:** Research seed
**Source:** TurboQuant (arXiv 2504.19874), better-router experiments E37-E38
**License target:** Apache 2.0 (clean-room implementation)

## Executive Summary

TinyQuant is a CPU-only vector quantization library for compressing
high-dimensional embeddings to 2-4 bits with near-zero score distortion.
It implements the TurboQuant algorithm (PolarQuant rotation + Lloyd-Max
scalar quantization + QJL residual correction) as a pure Python/NumPy
package suitable for pip installation. TinyQuant is a **storage codec**,
not a search accelerator: vectors are compressed on write, decompressed
to FP32 for search. A pluggable search backend interface allows callers
to substitute their own indexing (pgvector HNSW, FAISS, brute-force).

The implementation is derived from a clean-room ~750-line prototype that
was validated across experiments E37-E38 in the better-router project,
achieving 8x compression with Pearson rho=0.997 on gold corpus score
fidelity.

A future compiled core (Rust or C with Python bindings) is planned as a
performance optimization path, validated against the Python baseline.

---

## Motivation

### Why extract from better-router

The better-router project built a clean-room TurboQuant implementation
for experiments E37-E38. That code proved itself:

- 8x storage reduction at 4-bit quantization
- Near-perfect score fidelity (Pearson 0.997) for quality estimation
- Rank correlation 0.98+ across all tested corpus sizes
- Theoretically sound: distortion matches paper's Theorem 1 within 20%,
  inner product estimator is provably unbiased

But the implementation lives in an experiment support library
(`experiments/lib/turboquant_core.py`), tightly coupled to the experiment
harness, with no packaging, no versioning, and no public API contract.
Extracting it into a standalone library makes the compression capability
reusable for any system that stores embeddings — PostgreSQL-based context
databases, vector caches, offline evaluation corpora.

### Why not use existing implementations

The published TurboQuant community implementation (`0xSero/turboquant`)
is GPL-3.0 licensed and GPU-only (Triton kernels). Neither constraint is
acceptable:

- GPL-3.0 is incompatible with Apache 2.0 downstream consumers
- GPU requirement excludes the target deployment environment (CPU-only
  Docker containers running alongside LM Studio nodes)

The clean-room implementation was built solely from the published paper's
algorithms and mathematical definitions. No GPL-3.0 source code was
referenced during development. Apache 2.0 reference repositories
(PolarQuant, QJL) were used only for cross-validation of outputs.

---

## Prior Art and Provenance

### TurboQuant (arXiv 2504.19874)

Google Research, 2025. A two-stage vector compression algorithm:

**Stage 1 — PolarQuant:** Apply a random orthogonal rotation matrix Pi
to the input vector. After rotation, each coordinate follows a
predictable scaled Beta distribution (Lemma 1 in the paper). This
eliminates per-block normalization overhead and enables optimal scalar
quantization using a dimension-dependent Lloyd-Max codebook.

**Stage 2 — QJL (Quantized Johnson-Lindenstrauss):** Compute a 1-bit
random projection of the quantization residual (the difference between
the original and quantized vector). This residual correction preserves
inner products in expectation (unbiased estimator). The projection is
data-oblivious and adds exactly 1 bit per dimension of overhead.

**Key results from the paper:**

- KV cache compression: 6x+ at 3 bits, zero accuracy loss on long-context
  benchmarks
- Vector search: higher recall@k than product quantization (PQ) and
  RaBbitQ at equal bit budgets
- Distortion within a constant factor of Shannon's information-theoretic
  lower bound
- No retraining or fine-tuning required

### License landscape

| Repository | License | Used for |
|-----------|---------|----------|
| TurboQuant paper | Academic (arXiv) | Algorithm source |
| 0xSero/turboquant | GPL-3.0 | NOT referenced during implementation |
| PolarQuant (Apache 2.0) | Apache 2.0 | Codebook cross-validation only |
| QJL (Apache 2.0) | Apache 2.0 | Estimator output cross-validation only |

### Clean-room methodology

The implementation was built from:

1. Algorithm 1 (TurboQuant_mse) and Algorithm 2 (TurboQuant_prod)
   as published in the paper
2. The coordinate distribution formula (Lemma 1, Eq. 4)
3. Standard Lloyd-Max quantizer theory
4. Standard Johnson-Lindenstrauss projection theory

At no point was the GPL-3.0 implementation's source code read, copied,
or referenced. The validation test suite compares outputs against
PolarQuant and QJL (both Apache 2.0) as black-box oracles.

---

## Architecture

TinyQuant is organized in three layers, from low-level codec to
high-level search integration:

```text
┌─────────────────────────────────────────────────┐
│            Search Interface Layer                │
│  SearchBackend protocol (pluggable)              │
│  Default: brute-force FP32                       │
│  Pluggable: pgvector HNSW, FAISS, custom        │
├─────────────────────────────────────────────────┤
│            Corpus Layer                          │
│  CompressedCorpus container                      │
│  Batch compress/decompress                       │
│  Serialization (to/from bytes, files)            │
│  Metadata association                            │
├─────────────────────────────────────────────────┤
│            Core Layer                            │
│  LloydMaxCodebook (dimension-specific solver)    │
│  TurboQuantMSE (Algorithm 1: rotate + quantize) │
│  QJL (1-bit residual projection)                 │
│  TurboQuantProd (Algorithm 2: MSE + QJL)         │
│  CompressedVector (serialization format)          │
└─────────────────────────────────────────────────┘
```

### Core Layer

The core layer implements the paper's two algorithms as stateless
transformations on vectors.

**LloydMaxCodebook:** Solves the optimal scalar quantizer for the
dimension-dependent coordinate distribution. Given dimension `d` and
bit depth `b`, computes `2^b` centroids and `2^b - 1` decision
boundaries that minimize MSE distortion. Codebooks are computed once
per (dimension, bit_depth) pair and cached.

**TurboQuantMSE:** Algorithm 1 from the paper. Takes a vector,
normalizes it to unit norm (storing the original norm separately for
reconstruction), multiplies by a random orthogonal rotation matrix Pi,
quantizes each rotated coordinate using the Lloyd-Max codebook, and
packs the quantized indices into a byte array. On decompression, the
quantized coordinates are looked up, inverse-rotated, and rescaled by
the stored norm. The rotation matrix Pi is generated deterministically
from a seed for reproducibility.

**QJL:** The Quantized Johnson-Lindenstrauss residual corrector.
Computes the quantization residual (original minus reconstructed),
projects it onto a random 1-bit matrix S, and stores the sign bits.
The projection matrix S is also seed-deterministic.

**TurboQuantProd:** Algorithm 2. Combines TurboQuantMSE with QJL
for improved inner product estimation. The total storage per vector
is `b*d` bits (MSE) + `d` bits (QJL signs) = `(b+1)*d` bits. At
4-bit MSE, this is 5 bits per dimension total.

**CompressedVector:** The serialized form of a compressed vector.
Contains the quantized indices (packed bits), optional QJL sign bits,
the original vector norm (FP32, 4 bytes), and a format header
(bit depth, dimension, algorithm variant, seed).

### Corpus Layer

**CompressedCorpus:** A container for a collection of compressed
vectors. Supports:

- `add(vector, metadata)` — compress and append
- `add_batch(vectors, metadata_list)` — compress batch in parallel
- `decompress(index)` → FP32 vector
- `decompress_batch(indices)` → FP32 matrix
- `decompress_all()` → full FP32 matrix
- `serialize() → bytes` and `deserialize(bytes)` — round-trip to
  persistent storage
- `save(path)` and `load(path)` — file I/O convenience

The corpus stores compressed vectors contiguously in memory. Metadata
(IDs, timestamps, categories) is stored alongside but not compressed.

### Search Interface Layer

**SearchBackend protocol:**

```python
class SearchBackend(Protocol):
    def search(
        self,
        query: NDArray[np.float32],    # (d,) FP32 query vector
        corpus: CompressedCorpus,       # compressed vectors
        k: int,                         # number of results
    ) -> list[SearchResult]: ...

@dataclass
class SearchResult:
    index: int          # position in corpus
    score: float        # similarity score
    metadata: dict      # associated metadata
```

**Default implementation — BruteForceFP32:**
Decompresses all vectors in the corpus to FP32, computes cosine
similarity (or inner product / L2) against the query, returns top-k.
This is the baseline that matches the E37-E38 validated pattern.

**Pluggable implementations:**

- `PgVectorBackend` — implemented by TurboSwede. Decompresses to a
  temporary FP32 representation and delegates to pgvector HNSW.
- `FAISSBackend` — wraps FAISS for users who want GPU-accelerated
  search on the decompressed vectors.
- Custom backends implement the `SearchBackend` protocol.

The key invariant: **search always operates on decompressed FP32
vectors.** The backend decides how to index and query those vectors,
but TinyQuant's compression is transparent to the search algorithm.

---

## Quantitative Baseline

These numbers from E37-E38 define the performance contract that
TinyQuant must preserve:

### Compression Quality (4-bit)

| Metric | d=384 (MiniLM) | d=3072 (text-embedding-3-large) |
|--------|---------------|-------------------------------|
| Storage compression | 7.7x | 8.0x |
| Rank correlation (Spearman) | 0.99 | 0.9945 |
| Gold score fidelity (Pearson) | 0.992 | **0.997** |
| Compress latency (CPU) | 0.08 ms/vec | 7 ms/vec |

### Top-1 Agreement Scaling Law

Top-1 agreement degrades as O(1/sqrt(n)) with corpus size. This is a
fundamental property of quantization noise shuffling vectors within a
similarity epsilon-ball:

| Corpus size | d=384 top-1 (4-bit) | d=3072 top-1 (4-bit) |
|------------|--------------------|--------------------|
| 200-500 | 0.72-0.78 | 0.80 |
| 1000 | 0.47 | ~0.65 |
| 3000 | ~0.25 | 0.565 |
| 5000 | 0.085 | 0.150 |

**Implication:** TinyQuant is a storage codec, not a precision retrieval
tool. Callers that need exact top-1 must decompress and search at FP32.
Callers that need only score fidelity (gold corpus) or approximate
neighborhood (top-5 overlap 80%+) can tolerate the quantization noise.

### Rejected Approaches

**Compressed-domain search (CPU):** TurboQuant's QJL inner product
estimator requires O(d^2) matrix-vector multiply per query. At d=3072,
this is 90-1000x slower than FP32 brute-force on CPU. Definitively
rejected in E37. TinyQuant does not expose compressed-domain search.

**3-bit and 2-bit quantization:** Higher compression ratios (10.6x and
14.8x respectively) but significantly worse recall and rank correlation.
4-bit is the sweet spot for the storage-fidelity tradeoff. TinyQuant
supports 2-4 bit depths but documents 4-bit as the recommended default.

---

## API Surface

### Core API

```python
from tinyquant_cpu import TinyQuantMSE, TinyQuantProd, CompressedCorpus

# Initialize compressor for a specific dimension and bit depth
compressor = TinyQuantProd(dimension=3072, bits=4, seed=42)

# Compress a single vector
compressed = compressor.compress(vector)  # NDArray → CompressedVector

# Decompress back to FP32
restored = compressor.decompress(compressed)  # CompressedVector → NDArray

# Batch operations
corpus = CompressedCorpus(compressor)
corpus.add_batch(vectors, metadata_list)

# Serialize to bytes for storage
data = corpus.serialize()
corpus2 = CompressedCorpus.deserialize(data, compressor)

# Search via pluggable backend
from tinyquant_cpu.search import BruteForceFP32
backend = BruteForceFP32(metric="cosine")
results = backend.search(query_vector, corpus, k=10)
```

### Search Backend Protocol

```python
from tinyquant_cpu.search import SearchBackend

class MyCustomBackend(SearchBackend):
    def search(self, query, corpus, k):
        # Decompress candidates
        vectors = corpus.decompress_all()
        # Use your own indexing...
        return results
```

### Serialization Format (CompressedCorpus)

```text
Header (16 bytes):
  magic: b"TQNT" (4 bytes)
  version: uint16
  algorithm: uint8 (0=MSE, 1=Prod)
  bits: uint8
  dimension: uint32
  count: uint32

Per vector:
  norm: float32 (4 bytes)
  quantized_indices: ceil(bits * dimension / 8) bytes
  qjl_signs: ceil(dimension / 8) bytes (Prod only)

Footer:
  metadata_json: variable length
  metadata_offset: uint64
```

---

## Future: Compiled Core

### Motivation

The pure Python/NumPy implementation is the v1 baseline. A compiled
core becomes valuable when:

- Batch decompress latency matters (e.g., decompressing 1000 vectors
  at d=3072 takes ~7 seconds in NumPy)
- Compress latency on the write path needs to drop below 1ms/vec
  for real-time ingestion
- Memory efficiency matters (NumPy intermediate allocations during
  rotation and quantization)

### Target Operations

| Operation | NumPy baseline | Compiled target | Notes |
|-----------|---------------|----------------|-------|
| Lloyd-Max iteration | ~100ms (one-time) | ~10ms | Only runs at init |
| Rotate (Pi @ x) | ~3ms at d=3072 | ~0.3ms | BLAS-bound |
| Quantize (codebook lookup) | ~0.5ms | ~0.05ms | Simple table lookup |
| QJL projection | ~3ms | ~0.3ms | BLAS-bound |
| Batch decompress (1000 vecs) | ~7s (estimated) | ~0.7s | Parallelizable |

### Implementation Path

1. **Rust with PyO3** (recommended): Write core operations in Rust,
   expose via PyO3 bindings. Rust's `ndarray` crate provides
   NumPy-compatible array operations. BLAS integration via `openblas`
   or `intel-mkl` for matrix operations.

2. **C with cffi** (alternative): Lower-level, more portable, but
   requires manual memory management. Suitable if the library needs
   to be consumed from non-Python callers (Go, Node.js).

### Validation Against Python Baseline

The compiled core must produce bit-identical output to the Python
implementation for the same inputs and seeds. The validation strategy:

- Property-based tests: unbiased estimator (statistical test, p>0.01)
- Round-trip fidelity: compress → decompress → compare to original
- Cross-implementation oracle: Python output == compiled output for
  a fixed set of test vectors at each supported (dimension, bits) pair
- Performance regression: compiled must be faster on every operation
- Distortion bounds: must match Theorem 1 values within 20%

---

## Validation Strategy

### Unit Tests

- **Codebook correctness:** Lloyd-Max centroids minimize MSE for the
  theoretical coordinate distribution at each dimension
- **Round-trip fidelity:** `decompress(compress(x))` preserves norm
  and direction within theoretical distortion bounds
- **Unbiasedness:** QJL inner product estimator is unbiased (statistical
  test over 10,000 random vector pairs, p>0.01)
- **Deterministic seeds:** Same input + same seed = same output, always

### Property-Based Tests

- **Distortion bounds:** Measured MSE matches Theorem 1 prediction
  within 20% across dimensions 128, 384, 768, 1536, 3072
- **Compression ratio:** Actual bytes per vector matches theoretical
  `ceil((bits + 1) * dimension / 8) + 4` (norm) + header
- **Scaling law:** Top-1 agreement degrades as O(1/sqrt(n)) on
  synthetic corpora of increasing size

### Cross-Validation

- Compare codebook centroids against PolarQuant (Apache 2.0) reference
  implementation for dimensions 128, 384, 768
- Compare QJL estimator outputs against QJL (Apache 2.0) reference
  implementation for random input pairs
- Both comparisons are black-box output checks, not code-level

### Integration Tests

- Serialize → deserialize round-trip preserves all vectors and metadata
- CompressedCorpus search results match brute-force FP32 search on the
  decompressed vectors (validates that the search interface doesn't
  introduce its own distortion)
- File I/O: save → load → search produces identical results
