# TinyQuant

TinyQuant is a proposed CPU-only vector quantization library for compressing
high-dimensional embedding vectors to low-bit representations while preserving
useful similarity scores.

The current repository is still research-first and documentation-first. The
implementation is not shipped yet. What exists today is the project structure,
the documentation system, and the initial research synthesis that defines what
TinyQuant should become.

## Project goal

TinyQuant is being shaped as a clean-room, Apache-2.0-friendly storage codec
for embeddings.

Its intended role is:

- compress embeddings on write
- store them efficiently
- decompress them back to FP32 for search
- let a pluggable backend handle retrieval

It is explicitly not being positioned as a vector database or as a
compressed-domain ANN engine.

## Why this project exists

The current research basis comes from a validated clean-room prototype built
for better-router experiments. That research suggests TinyQuant can provide:

- roughly 8x storage compression at 4-bit settings
- near-zero distortion for score-oriented uses such as gold-corpus similarity
- a CPU-only deployment story suitable for ordinary Python services and Docker
  environments
- a reusable packaging boundary instead of leaving the codec trapped inside an
  experiment harness

The project is also motivated by licensing and deployment constraints:

- community GPU implementations are not a fit for CPU-only targets
- GPL-licensed implementations are not acceptable for the intended downstream
  use cases
- TinyQuant therefore aims for a clean-room implementation grounded in the
  published research and validated against permissively licensed references

## Current status

TinyQuant is currently at the research and architecture stage.

The documented intended v1 shape is:

- pure Python and NumPy baseline
- deterministic compression and decompression behavior
- a corpus container for batch workflows and serialization
- a pluggable search backend protocol
- a future compiled core path, validated against the Python baseline

The most important current project documents are:

- [docs/research/tinyquant-research/README.md](docs/research/tinyquant-research/README.md)
- [docs/research/tinyquant-research/better-router-integration.md](docs/research/tinyquant-research/better-router-integration.md)
- [docs/research/vector-quantization-paper-synthesis.md](docs/research/vector-quantization-paper-synthesis.md)
- [docs/research/turbo-quant-deep-research-report.md](docs/research/turbo-quant-deep-research-report.md)
- [docs/research/turboquant-integration.md](docs/research/turboquant-integration.md)
- [docs/entities/TinyQuant.md](docs/entities/TinyQuant.md)
- [docs/design/storage-codec-architecture.md](docs/design/storage-codec-architecture.md)

## Research lineage

The current project framing comes from a research line that runs through:

- QJL for residual-based inner-product preservation
- PolarQuant for random-preconditioning and zero-overhead scalar quantization
- TurboQuant for the combined two-stage codec story and the strongest
  high-dimensional compression framing

TinyQuant is not trying to reproduce those projects wholesale. It is trying to
adapt that line of work into a clean-room, CPU-first, embedding-storage library
with a pragmatic systems boundary.

## Core design principles

### 1. Storage codec first

TinyQuant should optimize embedding storage, not try to replace search systems.
Search should operate on decompressed FP32 vectors or on materialized FP32
representations maintained by downstream systems.

### 2. CPU-only baseline

The first useful version should run well in normal CPU-only Python
environments. Any future compiled core is an optimization path, not the
definition of the project.

### 3. Clean-room provenance

The implementation should remain clearly separated from GPL-only reference
implementations. The repository should preserve an evidence trail showing that
the algorithmic design came from the paper and from non-copyleft validation
sources.

### 4. Per-workload rollout

Not every collection or embedding workload should necessarily use the same
compression policy. The current research already points toward per-collection
compression decisions in downstream integrations.

## Expected downstream use

The main researched integration target so far is the better-router ecosystem
through TurboSwede:

- TinyQuant compresses vectors before durable storage
- TurboSwede owns storage and materialized decompression strategy
- better-router consumes retrieval results through TurboSwede rather than
  calling TinyQuant directly

That integration model keeps TinyQuant small and reusable while still allowing
larger systems to benefit from the storage savings.

## Repository layout

| Path | Purpose |
| --- | --- |
| `docs/` | compiled knowledge layer and Obsidian wiki |
| `docs/research/` | raw research inputs and idea files |
| `scripts/` | repository automation and verification helpers |
| `.githooks/` | versioned Git hooks for local enforcement |
| `AGENTS.md` | repository operating rules for agents |
| `CONCEPTS.md` | root glossary for non-Obsidian documentation |
| `CLAUDE.md` | lightweight pointer back to `AGENTS.md` |

## Documentation system

This repository uses two markdown modes:

- inside `docs/`: Obsidian-flavored markdown for the LLM-maintained wiki
- outside `docs/`: ordinary markdown with strict markdownlint discipline

The documentation system follows the LLM Wiki model described in
[docs/research/llm-wiki.md](docs/research/llm-wiki.md). In TinyQuant, the wiki
under `docs/` is the compiled knowledge layer, while the raw evidence layer
includes both `docs/research/` and the rest of the repository outside `docs/`.

## Verification

This repository uses a versioned pre-commit hook in `.githooks/pre-commit`.

Install or refresh the local hook path with:

```powershell
git config core.hooksPath .githooks
```

Run the same verification manually with:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\pre-commit.ps1
```

The current checks enforce the documentation boundary and the root documentation
baseline while the project is still in its early documentation-heavy phase.

## Related files

- [AGENTS.md](AGENTS.md)
- [CONCEPTS.md](CONCEPTS.md)
- [docs/README.md](docs/README.md)
- [docs/index.md](docs/index.md)
- [scripts/README.md](scripts/README.md)
