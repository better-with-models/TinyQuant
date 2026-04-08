---
title: Class Specifications
tags:
  - classes
  - api
  - meta
date-created: 2026-04-08
status: active
category: design
---

# Class Specifications

> [!info] Purpose
> Planned Python class files for TinyQuant, organized by package. Each page
> specifies the class responsibility, members, invariants, and relationships.

## Index by package

### `tinyquant.codec` — Codec Context

| Class | File | Role | Page |
|-------|------|------|------|
| `CodecConfig` | `codec_config.py` | Immutable configuration value object | [[classes/codec-config|CodecConfig]] |
| `RotationMatrix` | `rotation_matrix.py` | Deterministic orthogonal transform | [[classes/rotation-matrix|RotationMatrix]] |
| `Codebook` | `codebook.py` | Quantization lookup table | [[classes/codebook|Codebook]] |
| `CompressedVector` | `compressed_vector.py` | Codec output value object | [[classes/compressed-vector|CompressedVector]] |
| `Codec` | `codec.py` | Stateless compression/decompression service | [[classes/codec|Codec]] |
| *(private)* | `_quantize.py` | Low-level quantization helpers | [[classes/quantize-internal|_quantize (internal)]] |

### `tinyquant.corpus` — Corpus Context

| Class | File | Role | Page |
|-------|------|------|------|
| `Corpus` | `corpus.py` | Aggregate root for stored vectors | [[classes/corpus|Corpus]] |
| `VectorEntry` | `vector_entry.py` | Identity-bearing vector entity | [[classes/vector-entry|VectorEntry]] |
| `CompressionPolicy` | `compression_policy.py` | Policy enum governing write path | [[classes/compression-policy|CompressionPolicy]] |
| *(events)* | `events.py` | Domain event dataclasses | [[classes/corpus-events|Corpus Events]] |

### `tinyquant.backend` — Backend Protocol Context

| Class | File | Role | Page |
|-------|------|------|------|
| `SearchBackend` | `protocol.py` | Protocol defining search contract | [[classes/search-backend|SearchBackend]] |
| `SearchResult` | `protocol.py` | Result value object | [[classes/search-result|SearchResult]] |
| `BruteForceBackend` | `brute_force.py` | Reference implementation | [[classes/brute-force-backend|BruteForceBackend]] |
| `PgvectorAdapter` | `adapters/pgvector.py` | Wire-format adapter | [[classes/pgvector-adapter|PgvectorAdapter]] |

### Shared

| Symbol | File | Role | Page |
|--------|------|------|------|
| Type aliases | `_types.py` | `VectorId`, `ConfigHash`, array aliases | [[classes/shared-types|Shared Types]] |

## Reading conventions

Each class page follows a consistent structure:

1. **Responsibility** — one-sentence statement of what the class owns
2. **File path** — where the class lives in the source layout
3. **Category** — value object, entity, aggregate root, domain service, or protocol
4. **Fields / Properties** — typed members with invariants
5. **Methods** — signature, purpose, preconditions, postconditions
6. **Invariants** — rules that must always hold
7. **Relationships** — which other classes this one depends on or produces
8. **Test file** — corresponding test module

## See also

- [[design/architecture/namespace-and-module-structure|Namespace and Module Structure]]
- [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]]
- [[design/domain-layer/ubiquitous-language|Ubiquitous Language]]
- [[design/architecture/file-and-complexity-policy|File and Complexity Policy]]
