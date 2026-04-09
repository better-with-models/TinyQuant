# TinyQuant

CPU-only vector quantization codec for compressing high-dimensional
embedding vectors to low-bit representations while preserving useful
similarity scores.

TinyQuant compresses embeddings on write, stores them efficiently, and
decompresses them back to FP32 for search. A pluggable backend protocol
handles retrieval.

## Installation

```bash
pip install tinyquant
```

For PostgreSQL + pgvector support:

```bash
pip install tinyquant[pgvector]
```

## Quick start

```python
import numpy as np
from tinyquant.codec import Codec, CodecConfig, Codebook

# Configure: 4-bit quantization with residual correction
config = CodecConfig(bit_width=4, dimension=768, seed=42)
codec = Codec()

# Train a codebook from representative vectors
training_data = np.random.default_rng(0).standard_normal((1000, 768)).astype(np.float32)
codebook = codec.build_codebook(training_data, config)

# Compress and decompress
vector = training_data[0]
compressed = codec.compress(vector, config, codebook)
restored = codec.decompress(compressed, config, codebook)
```

## Corpus and search

```python
from tinyquant.corpus import Corpus, CompressionPolicy
from tinyquant.backend import BruteForceBackend

# Build a corpus with compressed storage
corpus = Corpus("my-corpus", config, codebook, CompressionPolicy.COMPRESS)
for i, vec in enumerate(training_data):
    corpus.insert(f"vec-{i}", vec)

# Decompress and search
backend = BruteForceBackend()
backend.ingest(corpus.decompress_all())
results = backend.search(training_data[42], top_k=10)
for r in results:
    print(f"{r.vector_id}: {r.score:.4f}")
```

## Key properties

- **~8x compression** at 4-bit without residuals; ~1.6x with FP16
  residual correction (Pearson rho >= 0.995)
- **Deterministic** — same inputs always produce byte-identical output
- **CPU-only** — pure Python + NumPy, no GPU required
- **Pluggable backends** — BruteForceBackend included, pgvector adapter
  available
- **Three compression policies** — COMPRESS, PASSTHROUGH, FP16

## Research lineage

TinyQuant adapts ideas from published research on random preconditioning
(PolarQuant), residual-based inner-product preservation (QJL), and
two-stage scalar quantization (TurboQuant) into a clean-room,
Apache-2.0-licensed implementation.

## Repository layout

| Path | Purpose |
| --- | --- |
| `src/tinyquant/` | Library source code |
| `src/tinyquant/codec/` | Codec, config, codebook, compressed vector, rotation |
| `src/tinyquant/corpus/` | Corpus aggregate, compression policies, domain events |
| `src/tinyquant/backend/` | Search backend protocol and implementations |
| `tests/` | Unit, integration, E2E, and calibration tests |
| `docs/` | Obsidian wiki with design docs, research, and specs |

## Development

```bash
pip install -e ".[dev]"
ruff check . && ruff format --check .
mypy --strict .
pytest --cov=tinyquant
```

## License

Apache-2.0

## Related documentation

- [Design: Storage Codec Architecture](docs/design/storage-codec-architecture.md)
- [Research Synthesis](docs/research/vector-quantization-paper-synthesis.md)
- [Validation Plan](docs/qa/validation-plan/README.md)
