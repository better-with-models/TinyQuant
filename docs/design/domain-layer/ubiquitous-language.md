---
title: Ubiquitous Language
tags:
  - design
  - domain
  - language
date-created: 2026-04-08
status: active
category: design
source-count: 6
---

# Ubiquitous Language

> [!info] Purpose
> Canonical vocabulary for TinyQuant. Every public API surface, test name, and
> document should use these terms consistently.

## Core terms

| Term | Definition | Context boundary |
|------|-----------|-----------------|
| **Vector** | A fixed-length array of FP32 values representing an embedding | Codec, Corpus |
| **Compressed vector** | The low-bit representation produced by the codec from an input vector | Codec |
| **Codebook** | A lookup structure that maps quantized indices back to representative values | Codec |
| **Rotation matrix** | A random orthogonal transformation applied before quantization to precondition coordinate distributions | Codec |
| **Residual** | The difference between a vector and its stage-1 reconstruction, preserved via a lightweight projection | Codec |
| **Seed** | A deterministic random seed that controls rotation matrix generation, ensuring reproducible transforms | Codec |
| **Bit width** | The number of bits per quantized coordinate (e.g. 4-bit is the practical default) | Codec |
| **Codec** | The stateless compression/decompression engine; owns quantization, rotation, residual correction, and serialization | Codec |
| **Corpus** | A named, persistent container of compressed vectors with associated metadata | Corpus |
| **Collection** | A downstream consumer's logical grouping of embeddings; one collection maps to one corpus with one compression policy | Corpus |
| **Compression policy** | A per-collection directive that selects the compression mode: `compress`, `passthrough`, or `fp16` | Corpus |
| **Backend** | An external search system that consumes decompressed FP32 vectors for similarity retrieval | Backend protocol |
| **Decompression** | The inverse of compression: reconstructing an FP32 vector from its compressed representation | Codec |
| **Score fidelity** | How closely similarity scores computed on decompressed vectors match scores on the original FP32 vectors | Quality (cross-cutting) |
| **Pearson rho** | The correlation metric used to measure score fidelity; target is near 0.997 for gold-corpus workloads | Quality (cross-cutting) |

## Overloaded terms to watch

| Term | TinyQuant meaning | Other common meaning | Resolution |
|------|-------------------|---------------------|------------|
| **Index** | Wiki catalog page or codebook index | ANN search index | TinyQuant does not build ANN indexes; use "codebook" or "wiki index" to disambiguate |
| **Quantization** | The full pipeline: rotation, scalar quantization, residual correction | Just rounding to fewer bits | Always specify stage when precision matters: "stage-1 quantization" vs "full codec pass" |
| **Compression** | The complete codec write path including rotation, quantization, residual, and serialization | Generic data compression (gzip, etc.) | Prefer "codec compression" or "vector compression" when context is ambiguous |
| **Collection** | A downstream logical grouping with a compression policy | A database table or index | Clarify as "embedding collection" in integration docs |

## Term evolution rules

> [!tip] When in doubt
> If a new concept does not fit an existing term, add it here before using it
> in code or documentation. Do not silently overload an existing term.

- New terms should be proposed in a design document before appearing in public API
- Deprecated terms should be marked here with a strikethrough and redirect

## See also

- [[domain-layer/context-map|Context Map]]
- [[domain-layer/aggregates-and-entities|Aggregates and Entities]]
- [[storage-codec-architecture]]
