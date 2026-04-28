<div align="center">

<picture>
  <source
    media="(prefers-color-scheme: dark)"
    srcset="../docs/assets/tinyquant-logo-dark-transparent.png">
  <img
    src="../docs/assets/tinyquant-logo-light-transparent.png"
    alt="TinyQuant logo: a database with a downward arrow and theta-r annotation"
    width="420">
</picture>

# TinyQuant

*Rust-native vector quantization codec for embedding compression — CPU SIMD, optional GPU acceleration, and Python/TypeScript bindings.*

[![PyPI](https://img.shields.io/pypi/v/tinyquant-cpu.svg)](https://pypi.org/project/tinyquant-cpu/)
[![CI](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml/badge.svg)](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml)
[![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Coverage](https://img.shields.io/badge/coverage-90.95%25-brightgreen.svg)](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml)

</div>

> [!NOTE]
> **TinyQuant** is a Rust-native vector quantization codec that compresses
> high-dimensional embedding vectors to low-bit representations while
> preserving cosine similarity rankings. It combines random orthogonal
> preconditioning with two-stage scalar quantization and optional FP16
> residual correction to hit **8× compression at 4-bit** with Pearson
> ρ ≈ 0.998 and **95% top-5 recall** on real OpenAI embeddings.
>
> - **What it is:** a Rust library (with Python and TypeScript bindings) that
>   squeezes embedding vectors into 4-bit (or 2-bit) representations without
>   losing retrieval quality, with optional wgpu GPU acceleration for batch
>   workloads above 512 vectors.
> - **Who it's for:** teams running cosine-similarity search on embeddings and
>   paying for RAM or disk by the gigabyte.
> - **Headline number:** 8× compression at 95% top-5 recall on 1536-dim
>   OpenAI embeddings. 1 M vectors go from **5.7 GB to 732 MB**.

---

## At a glance

On a benchmark of **335 real embeddings** from OpenAI's
`text-embedding-3-small` (1536 dimensions), TinyQuant 4-bit achieves
**8× compression** with Pearson ρ = 0.998 and 95% top-5 recall —
reducing a 6 KB embedding to 768 bytes while preserving the similarity
rankings that drive retrieval quality.

| Method                      |  Bytes/vec | Compression | Pearson ρ | Top-5 Recall |
| :-------------------------- | ---------: | ----------: | --------: | -----------: |
| FP32 (baseline)             |      6,144 |          1× |    1.0000 |         100% |
| FP16                        |      3,072 |          2× |    1.0000 |         100% |
| uint8 scalar                |      1,544 |          4× |    1.0000 |         100% |
| **TinyQuant 4-bit**         |    **768** |      **8×** |**0.9981** |      **95%** |
| **TinyQuant 2-bit**         |    **384** |     **16×** |**0.9643** |      **85%** |
| TinyQuant 4-bit + residual  |      3,840 |        1.6× |    1.0000 |         100% |

For a corpus of 1 million 1536-dim vectors, TinyQuant 4-bit reduces
storage from **5.7 GB to 732 MB** with negligible loss in retrieval
quality.

![Compression vs. Fidelity](../experiments/quantization-benchmark/results/plots/compression_vs_fidelity.png)

See the [full benchmark report](../experiments/quantization-benchmark/REPORT.md)
for methodology, all 9 methods compared, throughput measurements, and
publication-quality plots.

---

<details>
<summary><b>Contents</b></summary>

- [Installation](#installation)
- [Language bindings](#language-bindings)
- [Quickstart](#quickstart)
- [How it works](#how-it-works)
- [Recipes](#recipes)
- [Key properties](#key-properties)
- [Research lineage](#research-lineage)
- [Repository layout](#repository-layout)
- [Development](#development)
- [Reproducing the benchmark](#reproducing-the-benchmark)
- [Contributing](#contributing)
- [License](#license)
- [Related documentation](#related-documentation)

</details>

---

## Installation

TinyQuant is published on PyPI as `tinyquant-cpu` and imports as
`tinyquant_cpu`. The current release is a Rust-backed fat wheel — no
pure-Python fallback.

| I want to...                                   | Install command                                    |
| :--------------------------------------------- | :------------------------------------------------- |
| Python (Rust-backed, current)                  | `pip install tinyquant-cpu`                        |
| Python + PostgreSQL/pgvector support            | `pip install "tinyquant-cpu[pgvector]"`            |
| Rust native crate                               | `cargo add tinyquant-core`                         |
| TypeScript / Node / Bun                         | `npm install @tinyquant/core`                      |
| Work on this repository                         | see the [Development](#development) section below  |

> [!TIP]
> The `[pgvector]` extra pulls in `psycopg[binary]>=3.1` for talking to a
> live PostgreSQL database. Python **3.12+** is required; the Rust workspace
> MSRV is **1.81**, with the optional `tinyquant-gpu-wgpu` crate carved out
> at **1.87** in its own CI lane.

---

## Language bindings

TinyQuant ships the same codec / corpus / backend surface across three
languages, versioned in lockstep via `rust/Cargo.toml`
`workspace.package.version`. All bindings delegate math to the shared
`tinyquant-core` Rust crate — there is no per-language reimplementation.

| Language    | Package                                                 | Install                          | Since |
| :---------- | :------------------------------------------------------ | :------------------------------- | :---- |
| Python      | [`tinyquant-cpu`](https://pypi.org/project/tinyquant-cpu/) ([![PyPI](https://img.shields.io/pypi/v/tinyquant-cpu.svg)](https://pypi.org/project/tinyquant-cpu/)) | `pip install tinyquant-cpu`      | Phase 24 |
| Rust        | [`tinyquant-core`](https://crates.io/crates/tinyquant-core) ([![crates.io](https://img.shields.io/crates/v/tinyquant-core.svg)](https://crates.io/crates/tinyquant-core)) | `cargo add tinyquant-core`       | Phase 22 |
| TypeScript  | [`tinyquant`](https://www.npmjs.com/package/tinyquant) ([![npm](https://img.shields.io/npm/v/tinyquant.svg)](https://www.npmjs.com/package/tinyquant)) | `npm install @tinyquant/core`    | Phase 25 |

All three packages guarantee byte-identical output on `config_hash`,
`Codebook::to_bytes`, and `CompressedVector::to_bytes`. See
[`COMPATIBILITY.md`](../COMPATIBILITY.md) for the supported cross-package
version pairs.

---

## Quickstart

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

<details>
<summary><b>What just happened?</b></summary>

1. **Configure** — `CodecConfig(bit_width=4, dimension=1536, seed=42)`
   sets the bit width (`4` → 8× compression), the vector dimension, and
   the RNG seed that controls the random rotation matrix. The seed makes
   the codec **deterministic** — same inputs always produce byte-identical
   output across all language bindings.
2. **Train** — `codec.build_codebook(training_vectors, config)` fits a
   small codebook on a representative sample of your data.
3. **Insert** — `Corpus(..., CompressionPolicy.COMPRESS)` creates a domain
   aggregate that compresses every vector on insert and tracks vector IDs.
4. **Decompress** — `corpus.decompress_all()` produces `(vector_id,
   fp32_vector)` pairs. The Rust core runs these in parallel via Rayon.
5. **Search** — `BruteForceBackend` performs exact cosine search and returns
   `SearchResult` objects with IDs and scores. Swap for `PgvectorAdapter`
   in production, or use the GPU path for large corpora.

</details>

---

## How it works

**The problem.** Naive scalar quantization crushes real embedding data because
coordinate distributions are skewed: a handful of dimensions carry most of
the signal and get mapped to the same bucket as noise.

**The trick.** Pre-multiplying each vector by a **random orthogonal matrix**
(derived via QR decomposition of a Gaussian matrix) uniformizes the coordinate
distribution without changing pairwise distances. After rotation, a single
shared scalar quantizer works well across **all** dimensions. This is the core
insight from [TurboQuant][] and [PolarQuant][].

**Two-stage refinement.** An optional **FP16 residual** on top of the 4-bit
coarse codebook gives you a separate point on the rate-distortion curve:
8× compression and ρ ≈ 0.998 without the residual; 1.6× compression and
ρ = 1.000 with it enabled — useful for reranking stages.

**Rust core with CPU and GPU paths.** The codec runs through
`tinyquant-core`, which dispatches SIMD kernels at runtime (AVX2+FMA on
x86_64, NEON on aarch64) and parallelizes batch compression with Rayon.
For workloads exceeding the **512-vector threshold**, the optional
`tinyquant-gpu-wgpu` crate offloads rotate/quantize/dequantize/residual
and corpus cosine search to WGSL compute shaders via wgpu, with lazy
pipeline caching to avoid per-call recompilation.

**Backend-agnostic.** The codec produces `CompressedVector` bytes; search
lives in a separate `SearchBackend` layer (`BruteForceBackend` for in-memory
exact search, `PgvectorAdapter` for PostgreSQL + pgvector, `WgpuBackend` for
GPU-accelerated corpus search), so you can plug TinyQuant into any retrieval
store without coupling storage to search.

[TurboQuant]: https://research.google/blog/turboquant-redefining-ai-efficiency-with-extreme-compression/
[PolarQuant]: https://arxiv.org/abs/2503.20024

---

## Recipes

Pick the config that matches your rate-distortion target:

| Config                                            | Bytes/vec | Compression |     ρ | Top-5 | When to use                |
| :------------------------------------------------ | --------: | ----------: | ----: | ----: | :------------------------- |
| `CodecConfig(bit_width=4)`                        |       768 |          8× | 0.998 |   95% | **Default** balance        |
| `CodecConfig(bit_width=2)`                        |       384 |         16× | 0.964 |   85% | Aggressive, needs rerank   |
| `CodecConfig(bit_width=4, residual_enabled=True)` |     3,840 |        1.6× | 1.000 |  100% | Reranking / exact-match    |

<details>
<summary><b>Single-vector compression</b></summary>

```python
import numpy as np
from tinyquant_cpu.codec import Codec, CodecConfig

config = CodecConfig(bit_width=4, dimension=768, seed=42)
codec = Codec()

training_data = np.random.default_rng(0).standard_normal((1000, 768)).astype(np.float32)
codebook = codec.build_codebook(training_data, config)

vector = training_data[0]
compressed = codec.compress(vector, config, codebook)
print(f"Original:   {vector.nbytes} bytes")
print(f"Compressed: {compressed.size_bytes} bytes")
print(f"Ratio:      {vector.nbytes / compressed.size_bytes:.1f}x")

restored = codec.decompress(compressed, config, codebook)
```

</details>

<details>
<summary><b>Batch compression (Rayon-parallel)</b></summary>

```python
# Parallelized via Rayon in the Rust core — byte-identical to serial output
vectors = np.random.default_rng(0).standard_normal((10_000, 768)).astype(np.float32)
compressed_batch = codec.compress_batch(vectors, config, codebook)
restored_batch = codec.decompress_batch(compressed_batch, config, codebook)
```

</details>

<details>
<summary><b>Tuning the rate–distortion tradeoff</b></summary>

```python
# Maximum compression: 16x at 2-bit
config_2bit = CodecConfig(bit_width=2, dimension=768, seed=42, residual_enabled=False)

# Practical sweet spot: 8x at 4-bit (rho >= 0.998)
config_4bit = CodecConfig(bit_width=4, dimension=768, seed=42, residual_enabled=False)

# Near-perfect fidelity: 4-bit + FP16 residual correction (1.6x, rho = 1.000)
config_4bit_res = CodecConfig(bit_width=4, dimension=768, seed=42, residual_enabled=True)
```

> [!WARNING]
> 2-bit compression drops top-5 recall to ~85%. Only use it when a
> reranking stage (FP16 residual, cross-encoder, exact search) sits
> downstream to recover the missing signal.

</details>

<details>
<summary><b>Compression policies</b></summary>

A `Corpus` can store vectors in three modes:

```python
from tinyquant_cpu.corpus import Corpus, CompressionPolicy

corpus_compressed = Corpus("c", config, codebook, CompressionPolicy.COMPRESS)
corpus_full       = Corpus("p", config, codebook, CompressionPolicy.PASSTHROUGH)
corpus_fp16       = Corpus("h", config, codebook, CompressionPolicy.FP16)
```

Policies let one corpus mix hot data (PASSTHROUGH), cold data (COMPRESS),
and middle-tier data (FP16) without rebuilding the codec.

</details>

<details>
<summary><b>Binary serialization (TQCV format)</b></summary>

`CompressedVector` instances serialize to the TQCV versioned binary format
(70-byte header + LSB-first packed indices + optional FP16 residual),
suitable for disk, network, or database storage. Mmap corpus files are
available via the Rust `tinyquant-io` crate for zero-copy access.

```python
from tinyquant_cpu.codec import CompressedVector

raw_bytes = compressed.to_bytes()
restored  = CompressedVector.from_bytes(raw_bytes)
```

</details>

<details>
<summary><b>PostgreSQL + pgvector backend</b></summary>

```python
import psycopg
from tinyquant_cpu.backend.adapters.pgvector import PgvectorAdapter

adapter = PgvectorAdapter(
    connection_factory=lambda: psycopg.connect("postgresql://user:pass@localhost/mydb"),
    table_name="embeddings",
)
adapter.ingest(corpus.decompress_all())
results = adapter.search(query_vector, top_k=10)
```

> [!IMPORTANT]
> Requires PostgreSQL with the `pgvector` extension installed. CI runs these
> tests against a live `pgvector/pgvector:pg17` container via testcontainers.

</details>

<details>
<summary><b>GPU acceleration (Rust only — wgpu)</b></summary>

The `tinyquant-gpu-wgpu` crate provides a `WgpuBackend` that offloads
batch compress/decompress and corpus cosine search to WGSL compute shaders.
It is workspace-internal (`publish = false`) and selected automatically
when a batch exceeds `GPU_BATCH_THRESHOLD` (512 vectors).

```rust
use tinyquant_gpu_wgpu::{WgpuBackend, BackendPreference};

// Default adapter (auto-select highest-performance GPU)
let backend = WgpuBackend::new().await?;

// Or select a specific backend:
let backend = WgpuBackend::new_with_preference(BackendPreference::Vulkan).await?;

// Warm up pipeline cache explicitly (optional — lazy otherwise)
backend.load_pipelines().await;

// GPU corpus search
let state = backend.prepare_corpus_for_device(&corpus_vecs).await?;
let results = backend.cosine_topk(&state, &query_vec, top_k).await?;
```

Available `BackendPreference` variants: `Auto`, `Vulkan`, `Metal`, `Dx12`,
`HighPerformance`, `LowPower`, `Software`.

</details>

---

## Key properties

- **8× compression** at 4-bit without residuals (ρ = 0.998, 95% recall)
- **16× compression** at 2-bit (ρ = 0.964, 85% recall)
- **Perfect fidelity** with optional FP16 residual correction (ρ = 1.000)
- **Deterministic** — same inputs produce byte-identical output across all language bindings and CPU architectures
- **Rust-native core** — `tinyquant-core`; CPU SIMD dispatch (AVX2+FMA / NEON) via `is_x86_feature_detected!` / ARMv8 base-ISA guarantee; Rayon parallel batch with determinism contract
- **Optional GPU acceleration** — `tinyquant-gpu-wgpu`; WGSL rotate/quantize/dequantize/residual and cosine-topk kernels; lazy `CachedPipelines`; `BackendPreference` adapter selection; auto-routes at ≥ 512 vectors
- **Multi-language** — Python fat wheel (`tinyquant-cpu`), TypeScript/Node (`@tinyquant/core`), Rust native (`tinyquant-core`), C ABI (`tinyquant-sys`)
- **Pluggable backends** — `BruteForceBackend` for in-process exact search; `PgvectorAdapter` for PostgreSQL + pgvector; `WgpuBackend` for GPU corpus search
- **Three compression policies** — COMPRESS, PASSTHROUGH, FP16, mixable within a corpus
- **TQCV serialization** — versioned 70-byte header + LSB-first bit-pack + optional FP16 residual; mmap corpus files via `tinyquant-io`
- **Calibration gates** — Pearson ρ and mean recall-at-k measured against OpenAI calibration fixtures; Criterion benchmarks with 10% regression budget
- **Fully typed** — `py.typed` marker, `mypy --strict` clean, TypeScript strict mode
- **Apache-2.0 licensed**

---

## Research lineage

TinyQuant adapts ideas from published research into a clean-room
implementation:

| Source              | Year | Key contribution                                                  |
| :------------------ | :--: | :---------------------------------------------------------------- |
| [**TurboQuant**][1] | 2025 | Random rotation + scalar quantization, no per-block norms         |
| [**PolarQuant**][2] | 2025 | QR-derived orthogonal preconditioning for coordinate uniformity   |
| [**QJL**][3]        | 2024 | Inner-product preservation bounds under aggressive quantization   |

[1]: https://research.google/blog/turboquant-redefining-ai-efficiency-with-extreme-compression/
[2]: https://arxiv.org/abs/2503.20024
[3]: https://arxiv.org/abs/2406.03482

---

## Repository layout

| Path                                          | Purpose                                                                     |
| :-------------------------------------------- | :-------------------------------------------------------------------------- |
| `rust/crates/tinyquant-core/`                 | Codec, corpus, backend trait, SIMD dispatch, Rayon parallel batch           |
| `rust/crates/tinyquant-io/`                   | TQCV serialization format and mmap corpus files                             |
| `rust/crates/tinyquant-gpu-wgpu/`             | Optional wgpu/WGSL GPU accelerator (`publish = false`, workspace-internal)  |
| `rust/crates/tinyquant-py/`                   | pyo3 Python extension — the engine behind `tinyquant-cpu`                   |
| `rust/crates/tinyquant-sys/`                  | C ABI via cbindgen                                                          |
| `rust/crates/tinyquant-cli/`                  | Standalone CLI binary                                                       |
| `rust/crates/tinyquant-js/`                   | napi-rs TypeScript/Node bindings (`@tinyquant/core`)                        |
| `rust/crates/tinyquant-bruteforce/`           | `BruteForceBackend` reference implementation                                |
| `rust/crates/tinyquant-pgvector/`             | PostgreSQL + pgvector ACL adapter                                           |
| `rust/crates/tinyquant-bench/`                | Criterion benchmarks + calibration quality gates                            |
| `tests/reference/tinyquant_py_reference/`     | Pure-Python frozen oracle — differential test reference (not shipped)       |
| `tests/parity/`                               | Cross-implementation parity suite (`pytest -m parity`)                     |
| `tests/`                                      | Python unit, integration, E2E, architecture, and calibration suites         |
| `experiments/`                                | Benchmarks and empirical evaluations                                        |
| `docs/`                                       | Obsidian wiki: design docs, research, SDLC plans, CI/CD specs               |

---

## Development

```bash
git clone https://github.com/better-with-models/TinyQuant.git
cd TinyQuant

# Python dev dependencies
pip install pytest pytest-cov hypothesis numpy ruff mypy build

# Lint and format
ruff check . && ruff format --check .

# Strict type check
mypy --strict .

# Run the full Python suite
pytest --cov=tinyquant_py_reference

# Cross-impl parity (Python ↔ Rust)
pytest -m parity -v

# Rust: lint and test
cd rust
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

The Python test suite includes **289 tests** covering unit, integration,
end-to-end, calibration, parity (cross-impl Python ↔ Rust), and
architecture-enforcement scenarios. Coverage is held above **90%** by CI
(**94%** for the codec subpackage). Live PostgreSQL + pgvector tests run
against a Docker container in CI via `testcontainers`.

> [!TIP]
> CI enforces three strict gates: `ruff check` / `ruff format --check`,
> `mypy --strict`, and `markdownlint-cli2` for all markdown outside `docs/`.
> The `docs/` vault uses Obsidian-flavored markdown under its own rules —
> see [`AGENTS.md`](../AGENTS.md) for the policy.

---

## Reproducing the benchmark

```bash
export OPENAI_API_KEY="your-key-here"
python experiments/quantization-benchmark/generate_embeddings.py
python experiments/quantization-benchmark/run_benchmark.py
python experiments/quantization-benchmark/generate_plots.py
```

This fetches 335 embeddings via the OpenAI API, benchmarks 9 quantization
methods, and produces plots and JSON results in
`experiments/quantization-benchmark/results/`.

---

## Contributing

Contributions are welcome. The short version:

1. **Issues and design discussions** — open a GitHub issue before starting
   non-trivial work so we can agree on scope.
2. **Follow the repo SDLC** — architecture decisions, coding standards, and
   pre-commit expectations live in [`AGENTS.md`](../AGENTS.md) and the
   `docs/design/` vault. Read [`CLAUDE.md`](../CLAUDE.md) if you're driving
   Claude Code or another LLM agent against this repo.
3. **Run the full gate locally** before pushing:
   `ruff check . && ruff format --check . && mypy --strict . && pytest --cov=tinyquant_cpu`
4. **Keep prose aligned** — edits to the project tagline, elevator pitch, or
   headline benchmark numbers must land in `README.md`, `.github/README.md`,
   `AGENTS.md`, and `CLAUDE.md` in the same commit.

---

## License

Apache-2.0. See [LICENSE](../LICENSE).

---

## Related documentation

- [Benchmark Report](../experiments/quantization-benchmark/REPORT.md) —
  full empirical evaluation in CS-paper format
- [CHANGELOG](../CHANGELOG.md) — release notes
- [Design: Storage Codec Architecture](../docs/design/storage-codec-architecture.md)
- [Design: GPU Acceleration](../docs/design/rust/gpu-acceleration.md)
- [Research: Vector Quantization Paper Synthesis](../docs/research/vector-quantization-paper-synthesis.md)
- [QA: Validation Plan](../docs/qa/validation-plan/README.md)
- [CI Plan](../docs/CI-plan/README.md) and [CD Plan](../docs/CD-plan/README.md)
