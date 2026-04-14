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

*CPU-only vector quantization codec for embedding storage compression.*

[![PyPI](https://img.shields.io/pypi/v/tinyquant-cpu.svg)](https://pypi.org/project/tinyquant-cpu/)
[![CI](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml/badge.svg)](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml)
[![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Coverage](https://img.shields.io/badge/coverage-90.95%25-brightgreen.svg)](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml)

</div>

> [!NOTE]
> **TinyQuant** is a CPU-only vector quantization codec that compresses
> high-dimensional embedding vectors to low-bit representations while
> preserving cosine similarity rankings. It combines random orthogonal
> preconditioning with two-stage scalar quantization and optional FP16
> residual correction to hit **8× compression at 4-bit** with Pearson
> ρ ≈ 0.998 and **95% top-5 recall** on real OpenAI embeddings.
>
> - **What it is:** a pure-Python + NumPy library that squeezes
>   embedding vectors into 4-bit (or 2-bit) representations without
>   losing retrieval quality.
> - **Who it's for:** teams running cosine-similarity search on
>   embeddings and paying for RAM or disk by the gigabyte.
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
`tinyquant_cpu` (following the convention used by `torch-cpu`,
`tensorflow-cpu`, and `onnxruntime-gpu`).

> [!IMPORTANT]
> **Phase 23 reference demotion.** `tinyquant-cpu==0.1.1` is the **last
> pure-Python release**. The pure-Python implementation has been demoted
> to a test-only reference under `tests/reference/tinyquant_py_reference/`
> and is no longer shipped from this tree. Phase 24 reclaims the
> `tinyquant-cpu` name on PyPI with a Rust-backed fat wheel at
> `0.2.0+` — same import path (`import tinyquant_cpu`), same public API
> surface, different engine.

| I want to...                                   | Install command                                    |
| :--------------------------------------------- | :------------------------------------------------- |
| Pin the last pure-Python release               | `pip install tinyquant-cpu==0.1.1`                 |
| Use PostgreSQL + pgvector on the `0.1.x` line  | `pip install "tinyquant-cpu[pgvector]==0.1.1"`     |
| Phase 24 Rust-backed fat wheel (when released) | `pip install 'tinyquant-cpu>=0.2.0'`               |
| Work on this repository                        | see the [Development](#development) section below  |

> [!TIP]
> The `[pgvector]` extra on `0.1.1` pulls in `psycopg[binary]>=3.1` for
> talking to a live PostgreSQL database. Python **3.12+** is required.
> The repository itself is no longer a buildable package — dev
> dependencies are installed directly.

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
   the codec **deterministic** — same inputs always produce byte-
   identical output.
2. **Train** — `codec.build_codebook(training_vectors, config)` fits a
   small codebook (a few hundred reference points in rotated space) on
   a representative sample of your data. The codebook is the lookup
   table quantized indices will reference.
3. **Insert** — `Corpus(..., CompressionPolicy.COMPRESS)` creates a
   domain aggregate that compresses every vector on insert. The corpus
   tracks vector IDs, emits lifecycle events, and enforces the
   configured compression policy.
4. **Decompress** — `corpus.decompress_all()` walks the corpus and
   produces an iterable of `(vector_id, fp32_vector)` pairs suitable
   for any search backend.
5. **Search** — `BruteForceBackend` performs exact cosine search on
   the decompressed vectors and returns `SearchResult` objects with
   IDs and scores. Swap it for `PgvectorAdapter` in production.

</details>

---

## How it works

**The problem.** Naive scalar quantization (rounding each coordinate to
one of 16 levels) destroys inner products on real embedding data because
coordinate distributions are skewed: a handful of dimensions carry most
of the signal and get crushed into the same bucket as the noise.

**The trick.** Pre-multiplying each vector by a **random orthogonal
matrix** (derived via QR decomposition of a Gaussian matrix) uniformizes
the coordinate distribution without changing pairwise distances. After
rotation, a single shared scalar quantizer works well across **all**
dimensions. This is the core insight from
[TurboQuant][] and [PolarQuant][].

**Two-stage refinement.** TinyQuant optionally adds an **FP16 residual**
on top of the 4-bit coarse codebook. With the residual disabled you get
8× compression and ρ ≈ 0.998; with it enabled you get 1.6× compression
and ρ = 1.000 — a separate point on the rate-distortion curve that's
useful for reranking stages.

**Backend-agnostic.** The codec just produces `CompressedVector` bytes.
Search lives in a separate layer (`BruteForceBackend` for in-memory
exact search, `PgvectorAdapter` for PostgreSQL + pgvector), so you can
plug TinyQuant into any retrieval store without coupling storage to
search.

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

# Train a codebook from a representative sample
training_data = np.random.default_rng(0).standard_normal((1000, 768)).astype(np.float32)
codebook = codec.build_codebook(training_data, config)

# Compress one vector
vector = training_data[0]
compressed = codec.compress(vector, config, codebook)
print(f"Original:   {vector.nbytes} bytes")
print(f"Compressed: {compressed.size_bytes} bytes")
print(f"Ratio:      {vector.nbytes / compressed.size_bytes:.1f}x")

# Decompress
restored = codec.decompress(compressed, config, codebook)
```

</details>

<details>
<summary><b>Batch compression</b></summary>

```python
# Compress 10,000 vectors at once — vectorized for NumPy throughput
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

# COMPRESS: full TinyQuant compression on insert
corpus_compressed = Corpus("c", config, codebook, CompressionPolicy.COMPRESS)

# PASSTHROUGH: store FP32 unchanged (useful for hot data)
corpus_full = Corpus("p", config, codebook, CompressionPolicy.PASSTHROUGH)

# FP16: lossy half-precision (no codec overhead)
corpus_fp16 = Corpus("h", config, codebook, CompressionPolicy.FP16)
```

Policies let one corpus mix hot data (PASSTHROUGH), cold data
(COMPRESS), and middle-tier data (FP16) without rebuilding the codec.

</details>

<details>
<summary><b>Binary serialization</b></summary>

`CompressedVector` instances serialize to a compact versioned binary
format suitable for disk, network, or database storage:

```python
from tinyquant_cpu.codec import CompressedVector

raw_bytes = compressed.to_bytes()
# Save raw_bytes to disk, send over network, store in a BYTEA column...

restored = CompressedVector.from_bytes(raw_bytes)
```

The format is forward-compatible — future codec versions will be able
to read bytes written by today's version.

</details>

<details>
<summary><b>PostgreSQL + pgvector backend</b></summary>

```python
import psycopg
from tinyquant_cpu.backend.adapters.pgvector import PgvectorAdapter

def connection_factory():
    return psycopg.connect("postgresql://user:pass@localhost/mydb")

adapter = PgvectorAdapter(
    connection_factory=connection_factory,
    table_name="embeddings",
)

# Decompress TinyQuant vectors and ingest into pgvector
adapter.ingest(corpus.decompress_all())
results = adapter.search(query_vector, top_k=10)
```

> [!IMPORTANT]
> Requires PostgreSQL with the `pgvector` extension installed and a
> table with a matching `vector(DIM)` column. CI runs these tests
> against a live `pgvector/pgvector:pg17` container via testcontainers.

</details>

---

## Key properties

- **8× compression** at 4-bit without residuals (ρ = 0.998, 95% recall)
- **16× compression** at 2-bit (ρ = 0.964, 85% recall)
- **Perfect fidelity** with optional FP16 residual correction (ρ = 1.000)
- **Deterministic** — same inputs always produce byte-identical output
- **CPU-only** — pure Python + NumPy, no GPU required, no native deps
- **Pluggable backends** — `BruteForceBackend` included; `PgvectorAdapter`
  for production PostgreSQL + pgvector stores
- **Three compression policies** — COMPRESS, PASSTHROUGH, FP16, mixable
  within a single corpus
- **Versioned binary serialization** — compact, forward-compatible format
- **Fully typed** — `py.typed` marker, `mypy --strict` clean
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

- **TurboQuant** (Google Research, 2025) — random rotation combined with
  scalar quantization eliminates per-block normalization, achieving
  state-of-the-art compression for AI embeddings.
- **PolarQuant** (2025) — random orthogonal preconditioning via QR
  decomposition uniformizes coordinate distributions for better scalar
  quantization.
- **QJL** (2024) — theoretical grounding for inner-product preservation
  under aggressive quantization.

[1]: https://research.google/blog/turboquant-redefining-ai-efficiency-with-extreme-compression/
[2]: https://arxiv.org/abs/2503.20024
[3]: https://arxiv.org/abs/2406.03482

---

## Repository layout

| Path                                          | Purpose                                                              |
| :-------------------------------------------- | :------------------------------------------------------------------- |
| `rust/`                                       | Cargo workspace for the shipping Rust implementation                 |
| `tests/reference/tinyquant_py_reference/`     | Pure-Python reference implementation — test-only oracle (Phase 23+)  |
| `tests/parity/`                               | Cross-impl parity suite (`pytest -m parity`); Phase 24 wires `rs`    |
| `tests/`                                      | Unit, integration, E2E, architecture, and calibration suites         |
| `experiments/`                                | Benchmarks and empirical evaluations                                 |
| `docs/`                                       | Obsidian wiki: design docs, research, SDLC plans, CI/CD specs        |

---

## Development

```bash
git clone https://github.com/better-with-models/TinyQuant.git
cd TinyQuant

# Install dev dependencies directly — the tree is no longer a buildable package.
pip install pytest pytest-cov hypothesis numpy ruff mypy build

# Lint and format
ruff check . && ruff format --check .

# Strict type check
mypy --strict .

# Run the full suite against tests/reference/tinyquant_py_reference.
pytest --cov=tinyquant_py_reference

# Cross-impl parity scaffold (rs side skipped until Phase 24 fat wheel).
pytest -m parity -v
```

The test suite includes **214 tests** covering unit, integration,
end-to-end, calibration, and architecture-enforcement scenarios
against the reference implementation. Coverage is held above **90%**
by CI. Live PostgreSQL + pgvector tests run against a Docker container
in CI via `testcontainers`.

> [!TIP]
> CI enforces three strict gates: `ruff check` / `ruff format --check`,
> `mypy --strict`, and `markdownlint-cli2` for all markdown outside
> `docs/`. The `docs/` vault uses richer Obsidian-flavored markdown
> under its own rules — see [`AGENTS.md`](../AGENTS.md) for the policy.

---

## Reproducing the benchmark

The full benchmark from the [report](../experiments/quantization-benchmark/REPORT.md)
can be reproduced with:

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

## Contributing

Contributions are welcome. The short version:

1. **Issues and design discussions** — open a GitHub issue before
   starting non-trivial work so we can agree on scope.
2. **Follow the repo SDLC** — architecture decisions, coding standards,
   and pre-commit expectations live in [`AGENTS.md`](../AGENTS.md) and the
   `docs/design/` vault. Read [`CLAUDE.md`](../CLAUDE.md) if you're
   driving Claude Code or another LLM agent against this repo.
3. **Run the full gate locally** before pushing:
   `ruff check . && ruff format --check . && mypy --strict . && pytest --cov=tinyquant_cpu`
4. **Keep prose aligned** — edits to the project tagline, elevator
   pitch, or headline benchmark numbers must land in `README.md`,
   `AGENTS.md`, and `CLAUDE.md` in the same commit. See the
   "Cross-file prose alignment" section in `AGENTS.md`.

---

## License

Apache-2.0. See [LICENSE](../LICENSE).

---

## Related documentation

- [Benchmark Report](../experiments/quantization-benchmark/REPORT.md) —
  full empirical evaluation in CS-paper format
- [CHANGELOG](../CHANGELOG.md) — release notes
- [Design: Storage Codec Architecture](../docs/design/storage-codec-architecture.md)
- [Research: Vector Quantization Paper Synthesis](../docs/research/vector-quantization-paper-synthesis.md)
- [QA: Validation Plan](../docs/qa/validation-plan/README.md)
- [CI Plan](../docs/CI-plan/README.md) and [CD Plan](../docs/CD-plan/README.md)
