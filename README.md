# TinyQuant

*Rust-native vector quantization codec for embedding compression — CPU SIMD, optional GPU acceleration, and Python/TypeScript bindings.*

[![PyPI](https://img.shields.io/pypi/v/tinyquant-cpu.svg)](https://pypi.org/project/tinyquant-cpu/)
[![CI](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml/badge.svg)](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)

TinyQuant is a Rust-native vector quantization codec that compresses
high-dimensional embedding vectors to low-bit representations while
preserving cosine similarity rankings. It combines random orthogonal
preconditioning with two-stage scalar quantization and optional FP16
residual correction to hit 8× compression at 4-bit with Pearson ρ ≈ 0.998
and 95% top-5 recall on real OpenAI embeddings.

The Rust core ships CPU SIMD dispatch (AVX2/NEON), Rayon parallel batch
compression, an optional wgpu/WGSL GPU accelerator, and first-class bindings
for Python and TypeScript.

Research lineage: [TurboQuant], [PolarQuant], and [QJL].

[TurboQuant]: https://research.google/blog/turboquant-redefining-ai-efficiency-with-extreme-compression/
[PolarQuant]: https://arxiv.org/abs/2503.20024
[QJL]: https://arxiv.org/abs/2406.03482

---

## Why TinyQuant

On a benchmark of **335 real embeddings** from OpenAI's
`text-embedding-3-small` (1536 dimensions), TinyQuant 4-bit achieves
**8x compression** with Pearson rho = 0.998 and 95% top-5 recall —
reducing a 6 KB embedding to 768 bytes while preserving the
similarity rankings that drive retrieval quality.

| Method | Bytes/vec | Compression | Pearson rho | Top-5 Recall |
| --- | ---: | ---: | ---: | ---: |
| FP32 (baseline) | 6,144 | 1.0x | 1.0000 | 100% |
| FP16 | 3,072 | 2.0x | 1.0000 | 100% |
| uint8 scalar | 1,544 | 4.0x | 1.0000 | 100% |
| **TinyQuant 4-bit** | **768** | **8.0x** | **0.9981** | **95%** |
| **TinyQuant 2-bit** | **384** | **16.0x** | **0.9643** | **85%** |
| TinyQuant 4-bit + residual | 3,840 | 1.6x | 1.0000 | 100% |

For a corpus of 1 million vectors at dim 1536, TinyQuant 4-bit
reduces storage from **5.7 GB to 732 MB** with negligible loss in
retrieval quality.

See the [full benchmark report](experiments/quantization-benchmark/REPORT.md)
for methodology, all 9 methods compared, throughput measurements, and
publication-quality plots.

![Compression vs. Fidelity](experiments/quantization-benchmark/results/plots/compression_vs_fidelity.png)

---

## Installation

TinyQuant is published on PyPI as `tinyquant-cpu` (imports as `tinyquant_cpu`).
The current release is a Rust-backed fat wheel with no Python-only fallback.

```bash
pip install tinyquant-cpu          # current Rust-backed release
pip install "tinyquant-cpu[pgvector]"   # with PostgreSQL + pgvector support
```

For Rust projects:

```toml
[dependencies]
tinyquant-core = "*"
```

For Node/Bun/TypeScript:

```bash
npm install @tinyquant/core
```

**Requirements:** Python 3.12+, NumPy 2.x, Rust 1.87+ MSRV for the Rust crate.

---

## Quickstart

Compress, store, and search a corpus of embeddings in under 20 lines:

```python
import numpy as np
from tinyquant_cpu.codec import Codec, CodecConfig
from tinyquant_cpu.corpus import Corpus, CompressionPolicy
from tinyquant_cpu.backend import BruteForceBackend

# 1. Configure the codec: 4-bit quantization for 1536-dim vectors
config = CodecConfig(bit_width=4, dimension=1536, seed=42)
codec = Codec()

# 2. Train a codebook from representative vectors
training_vectors = np.random.default_rng(0).standard_normal((1000, 1536)).astype(np.float32)
codebook = codec.build_codebook(training_vectors, config)

# 3. Create a corpus that compresses on insert
corpus = Corpus("my-vectors", config, codebook, CompressionPolicy.COMPRESS)
for i, vec in enumerate(training_vectors):
    corpus.insert(f"vec-{i}", vec)

# 4. Decompress and search
backend = BruteForceBackend()
backend.ingest(corpus.decompress_all())
results = backend.search(training_vectors[42], top_k=5)
for r in results:
    print(f"{r.vector_id}: {r.score:.4f}")
```

---

## Basic Usage

### Single-vector compression

```python
import numpy as np
from tinyquant_cpu.codec import Codec, CodecConfig

config = CodecConfig(bit_width=4, dimension=768, seed=42)
codec = Codec()

# Train a codebook from a representative sample
training_data = np.random.default_rng(0).standard_normal((1000, 768)).astype(np.float32)
codebook = codec.build_codebook(training_data, config)

# Compress one vector
vector = training_data[0]
compressed = codec.compress(vector, config, codebook)
print(f"Original: {vector.nbytes} bytes")
print(f"Compressed: {compressed.size_bytes} bytes")
print(f"Ratio: {vector.nbytes / compressed.size_bytes:.1f}x")

# Decompress
restored = codec.decompress(compressed, config, codebook)
```

### Batch compression (Rayon-parallel on Rust side)

```python
# Compress 10,000 vectors at once — parallelized via Rayon in the Rust core
vectors = np.random.default_rng(0).standard_normal((10_000, 768)).astype(np.float32)
compressed_batch = codec.compress_batch(vectors, config, codebook)
restored_batch = codec.decompress_batch(compressed_batch, config, codebook)
```

### Tuning the rate-distortion tradeoff

```python
# Maximum compression: 16x at 2-bit
config_2bit = CodecConfig(bit_width=2, dimension=768, seed=42, residual_enabled=False)

# Practical sweet spot: 8x at 4-bit (rho >= 0.998)
config_4bit = CodecConfig(bit_width=4, dimension=768, seed=42, residual_enabled=False)

# Perfect fidelity: 4-bit + FP16 residual correction
config_4bit_res = CodecConfig(bit_width=4, dimension=768, seed=42, residual_enabled=True)
```

### Compression policies

A `Corpus` can store vectors in three modes:

```python
from tinyquant_cpu.corpus import Corpus, CompressionPolicy

# COMPRESS: full TinyQuant compression on insert
corpus_compressed = Corpus("c", config, codebook, CompressionPolicy.COMPRESS)

# PASSTHROUGH: store FP32 unchanged (useful for hot data)
corpus_full = Corpus("p", config, codebook, CompressionPolicy.PASSTHROUGH)

# FP16: lossy half-precision (no codec overhead)
corpus_fp16 = Corpus("h", config, codebook, CompressionPolicy.FP16)
```

### Serialization

`CompressedVector` instances serialize to a compact binary format
suitable for disk or network transfer:

```python
from tinyquant_cpu.codec import CompressedVector

raw_bytes = compressed.to_bytes()
restored = CompressedVector.from_bytes(raw_bytes)
```

### PostgreSQL + pgvector backend

```python
import psycopg
from tinyquant_cpu.backend.adapters.pgvector import PgvectorAdapter

adapter = PgvectorAdapter(
    connection_factory=lambda: psycopg.connect("postgresql://user:pass@localhost/mydb"),
    table_name="embeddings",
    dimension=768,
)
adapter.ingest(corpus.decompress_all())
results = adapter.search(query_vector, top_k=10)
```

---

## Key Properties

- **8x compression** at 4-bit without residuals (rho = 0.998, 95% recall)
- **16x compression** at 2-bit (rho = 0.964, 85% recall)
- **Perfect fidelity** with optional FP16 residual correction (rho = 1.000)
- **Deterministic** — same inputs always produce byte-identical output across all language bindings
- **Rust-native core** — `tinyquant-core` crate; CPU SIMD dispatch (AVX2/NEON) via runtime feature detection; Rayon parallel batch with determinism guarantee
- **Optional GPU acceleration** — `tinyquant-gpu-wgpu` crate; WGSL kernels for rotate/quantize/dequantize/residual and corpus cosine search; lazy pipeline caching; `BackendPreference` adapter selection; automatic routing above 512-vector threshold
- **Multi-language** — Python fat wheel (`tinyquant-cpu`), TypeScript/Node (`@tinyquant/core`), Rust native (`tinyquant-core`), C ABI (`tinyquant-sys`)
- **Pluggable backends** — `BruteForceBackend` for in-process exact search; `PgvectorAdapter` for PostgreSQL + pgvector
- **Three compression policies** — COMPRESS, PASSTHROUGH, FP16
- **Versioned binary serialization** — TQCV wire format with mmap corpus files (`tinyquant-io`)
- **Apache-2.0 licensed**

---

## Research Lineage

TinyQuant adapts ideas from published research into a clean-room
implementation:

- [**TurboQuant**] (Google Research, 2025) — Random rotation combined
  with scalar quantization eliminates per-block normalization,
  achieving state-of-the-art compression for AI embeddings.
- [**PolarQuant**] (2025) — Random orthogonal preconditioning via QR
  decomposition uniformizes coordinate distributions for better
  scalar quantization.
- [**QJL**] (Quantized Johnson-Lindenstrauss, 2024) — Theoretical
  grounding for inner-product preservation under aggressive
  quantization.

[**TurboQuant**]: https://research.google/blog/turboquant-redefining-ai-efficiency-with-extreme-compression/
[**PolarQuant**]: https://arxiv.org/abs/2503.20024
[**QJL**]: https://arxiv.org/abs/2406.03482

---

## Repository Layout

| Path | Purpose |
| --- | --- |
| `rust/crates/tinyquant-core/` | Codec, corpus, backend trait, SIMD dispatch (AVX2/NEON), Rayon batch |
| `rust/crates/tinyquant-io/` | TQCV serialization format, zero-copy mmap corpus files |
| `rust/crates/tinyquant-gpu-wgpu/` | Optional wgpu/WGSL GPU accelerator (Phase 27+) |
| `rust/crates/tinyquant-py/` | pyo3 Python extension — the engine behind `tinyquant-cpu` |
| `rust/crates/tinyquant-sys/` | C ABI via cbindgen |
| `rust/crates/tinyquant-cli/` | Standalone CLI binary |
| `rust/crates/tinyquant-js/` | napi-rs TypeScript/Node bindings |
| `rust/crates/tinyquant-bruteforce/` | `BruteForceBackend` reference implementation |
| `rust/crates/tinyquant-pgvector/` | PostgreSQL + pgvector ACL adapter |
| `rust/crates/tinyquant-bench/` | Criterion benchmarks + calibration quality gates |
| `tests/reference/tinyquant_py_reference/` | Pure-Python frozen oracle — differential test reference |
| `tests/parity/` | Cross-implementation parity suite (`pytest -m parity`) |
| `tests/` | Python unit, integration, E2E, calibration, and architecture suites (252 tests) |
| `experiments/` | Benchmarks and empirical evaluations |
| `docs/` | Obsidian wiki with design docs, research, and SDLC plans |

---

## Development

```bash
git clone https://github.com/better-with-models/TinyQuant.git
cd TinyQuant

# Python dev dependencies
pip install pytest pytest-cov hypothesis numpy ruff mypy build

# Lint and type check
ruff check . && ruff format --check .
mypy --strict .

# Python test suite (exercises tests/reference/tinyquant_py_reference)
pytest --cov=tinyquant_py_reference

# Rust: lint, test, cross-impl parity
cd rust
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

The Python test suite includes 252 tests covering unit, integration,
end-to-end, calibration, and architecture-enforcement scenarios.
Coverage is held above 90% by CI. Live PostgreSQL + pgvector tests run
against a Docker container in CI via `testcontainers`.

---

## Reproducing the Benchmark

```bash
export OPENAI_API_KEY="your-key-here"
python experiments/quantization-benchmark/generate_embeddings.py
python experiments/quantization-benchmark/run_benchmark.py
python experiments/quantization-benchmark/generate_plots.py
```

This fetches 335 embeddings via the OpenAI API, benchmarks 9
quantization methods, and produces plots and JSON results in
`experiments/quantization-benchmark/results/`.

---

## License

Apache-2.0. See [LICENSE](LICENSE).

---

## Related Documentation

- [Benchmark Report](experiments/quantization-benchmark/REPORT.md) —
  full empirical evaluation in CS-paper format
- [CHANGELOG](CHANGELOG.md) — release notes
- [Design: Storage Codec Architecture](docs/design/storage-codec-architecture.md)
- [Research Synthesis](docs/research/vector-quantization-paper-synthesis.md)
- [Validation Plan](docs/qa/validation-plan/README.md)
