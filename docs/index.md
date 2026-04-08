---
title: Wiki Index
tags:
  - meta
  - index
date-created: 2026-04-08
status: active
---

# Index

> [!info] Content catalog
> This page is the entry point for the TinyQuant wiki. Update it whenever new
> durable documentation is added.

## Sources

| Page | Summary | Date |
|------|---------|------|
| [[tinyquant-library-research]] | Source summary for the TinyQuant library research seed: codec scope, algorithm shape, quality baseline, and API direction | 2026-04-08 |
| [[tinyquant-better-router-integration]] | Source summary for the better-router and TurboSwede integration path, deployment order, and operational constraints | 2026-04-08 |
| [[vector-quantization-paper-synthesis]] | Cross-paper synthesis of QJL, PolarQuant, and TurboQuant, including the main algorithmic themes relevant to TinyQuant | 2026-04-08 |
| [[turbo-quant-deep-research-report]] | Deep research summary of TurboQuant's claims, deployment posture, and relationship to neighboring quantization methods | 2026-04-08 |
| [[turbo-quant-code-availability]] | Summary of official and community code availability across TurboQuant and related methods, with provenance implications for TinyQuant | 2026-04-08 |
| [[turboquant-integration]] | System-integration summary for bringing TurboQuant-style compression into the better-router stack | 2026-04-08 |

## Entities

| Page | Summary | Date |
|------|---------|------|
| [[TinyQuant]] | Project entity page covering TinyQuant as a CPU-only storage codec for embedding compression with pluggable search integration | 2026-04-08 |
| [[TurboQuant]] | Main upstream method entity that anchors TinyQuant's algorithmic lineage and quality expectations | 2026-04-08 |
| [[PolarQuant]] | Upstream method entity for the random-preconditioning and zero-overhead quantization story | 2026-04-08 |
| [[QJL]] | Upstream method entity for the residual-correction and unbiased-inner-product lineage | 2026-04-08 |

## Concepts

| Page | Summary | Date |
|------|---------|------|
| [[storage-codec-vs-search-backend-separation]] | Core architectural boundary: TinyQuant owns compression and decompression, while search stays external and FP32 | 2026-04-08 |
| [[per-collection-compression-policy]] | Operational concept for deciding which collections should compress, pass through, or use alternative precision modes | 2026-04-08 |
| [[random-preconditioning-without-normalization-overhead]] | Core quantization concept: random transforms can remove normalization overhead and make low-bit storage practical | 2026-04-08 |
| [[two-stage-quantization-and-residual-correction]] | Core quantization concept: stage one captures most structure, stage two protects inner-product fidelity | 2026-04-08 |

## Design

| Page | Summary | Date |
|------|---------|------|
| [[design/domain-layer/README|Domain Layer]] | Domain-driven design analysis: bounded contexts, aggregates, ubiquitous language, invariants, and domain events | 2026-04-08 |
| [[design/domain-layer/ubiquitous-language|Ubiquitous Language]] | Canonical vocabulary for TinyQuant's public API, tests, and documentation | 2026-04-08 |
| [[design/domain-layer/context-map|Context Map]] | Bounded contexts (Codec, Corpus, Backend Protocol), subdomain classification, and integration styles | 2026-04-08 |
| [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]] | Tactical model: Corpus aggregate root, CodecConfig/CompressedVector value objects, Codec domain service | 2026-04-08 |
| [[design/domain-layer/domain-events|Domain Events]] | Business-meaningful events: CodebookTrained, CorpusCreated, VectorsInserted, CorpusDecompressed | 2026-04-08 |
| [[design/behavior-layer/README|Behavior Layer]] | BDD behavior specifications: Given/When/Then scenarios for TinyQuant's observable contracts | 2026-04-08 |
| [[design/behavior-layer/codec-compression|Codec Compression Behavior]] | Scenarios for the write path: determinism, index range, residual presence, dimension enforcement | 2026-04-08 |
| [[design/behavior-layer/codec-decompression|Codec Decompression Behavior]] | Scenarios for the read path: FP32 output, config safety, round-trip fidelity, residual benefit | 2026-04-08 |
| [[design/behavior-layer/corpus-management|Corpus Management Behavior]] | Scenarios for corpus lifecycle: config freezing, policy enforcement, cross-config rejection | 2026-04-08 |
| [[design/behavior-layer/score-fidelity|Score Fidelity Behavior]] | Quality contract scenarios: Pearson rho baseline, rank preservation, bit-width ordering | 2026-04-08 |
| [[design/behavior-layer/backend-protocol|Backend Protocol Behavior]] | Integration boundary scenarios: FP32-only handoff, adapter translation, error isolation | 2026-04-08 |
| [[design/architecture/README|Architecture Design Considerations]] | Binding design policies for TinyQuant implementation: TDD, SOLID, complexity, coherence, coupling, linting, typing, docstrings, namespaces | 2026-04-08 |
| [[design/architecture/test-driven-development|Test-Driven Development]] | Red-green-refactor policy: all code enters through failing tests, test levels, design signals | 2026-04-08 |
| [[design/architecture/solid-principles|SOLID Principles]] | Change-driven application of SRP, OCP, LSP, ISP, DIP to TinyQuant's codec/corpus/backend structure | 2026-04-08 |
| [[design/architecture/file-and-complexity-policy|File and Complexity Policy]] | One public class per file, CC max 7 per function, file length monitoring | 2026-04-08 |
| [[design/architecture/high-coherence|High Coherence]] | Conceptual, module, architectural, and runtime coherence enforced by package structure | 2026-04-08 |
| [[design/architecture/low-coupling|Low Coupling]] | Acyclic dependencies, value objects at boundaries, protocol-based contracts, no shared mutable state | 2026-04-08 |
| [[design/architecture/linting-and-tooling|Linting and Tooling]] | Strict ruff + mypy + pytest in CI, warnings as errors, no bare suppressions | 2026-04-08 |
| [[design/architecture/docstring-policy|Docstring Policy]] | PEP 257 Google-style docstrings on all public symbols, enforced by ruff rule D | 2026-04-08 |
| [[design/architecture/type-safety|Type Safety]] | mypy strict mode, full annotations, Protocol-based contracts, suppression justification policy | 2026-04-08 |
| [[design/architecture/namespace-and-module-structure|Namespace and Module Structure]] | Package layout mapping bounded contexts to Python packages with explicit public APIs | 2026-04-08 |
| [[storage-codec-architecture]] | Three-layer TinyQuant architecture: codec core, corpus container, and pluggable search backend boundary | 2026-04-08 |

## Class Specifications

| Page | Summary | Date |
|------|---------|------|
| [[classes/README|Class Specifications]] | Index of all planned Python classes by package with member and responsibility summaries | 2026-04-08 |
| [[classes/codec-config|CodecConfig]] | Immutable configuration value object: bit_width, seed, dimension, residual_enabled | 2026-04-08 |
| [[classes/rotation-matrix|RotationMatrix]] | Deterministic orthogonal transform value object with apply/apply_inverse | 2026-04-08 |
| [[classes/codebook|Codebook]] | Quantization lookup table value object with train/quantize/dequantize | 2026-04-08 |
| [[classes/compressed-vector|CompressedVector]] | Codec output value object: indices, residual, config_hash, serialization | 2026-04-08 |
| [[classes/codec|Codec]] | Stateless domain service: compress, decompress, build_codebook, build_rotation | 2026-04-08 |
| [[classes/quantize-internal|_quantize (internal)]] | Private low-level quantization helpers: scalar_quantize, compute_residual | 2026-04-08 |
| [[classes/corpus|Corpus]] | Aggregate root: insert, insert_batch, decompress_all, config/policy enforcement | 2026-04-08 |
| [[classes/vector-entry|VectorEntry]] | Identity-bearing vector entity within a corpus | 2026-04-08 |
| [[classes/compression-policy|CompressionPolicy]] | Enum (compress/passthrough/fp16) governing corpus write-path behavior | 2026-04-08 |
| [[classes/corpus-events|Corpus Events]] | Domain event dataclasses: CorpusCreated, VectorsInserted, CorpusDecompressed, violation detection | 2026-04-08 |
| [[classes/search-backend|SearchBackend]] | Protocol defining the FP32-only search contract for external backends | 2026-04-08 |
| [[classes/search-result|SearchResult]] | Ranked result value object: vector_id + score | 2026-04-08 |
| [[classes/brute-force-backend|BruteForceBackend]] | Reference SearchBackend implementation using exhaustive cosine similarity | 2026-04-08 |
| [[classes/pgvector-adapter|PgvectorAdapter]] | Anti-corruption layer adapter for PostgreSQL + pgvector | 2026-04-08 |
| [[classes/shared-types|Shared Types]] | Type aliases: VectorId, ConfigHash, CorpusId, Vector, VectorBatch | 2026-04-08 |

## Behavior Specifications

| Page | Summary | Date |
|------|---------|------|
| [[behavior/README|Behavior README]] | Placeholder for future behavior specs and acceptance-criteria organization | 2026-04-08 |

## Specs

| Page | Summary | Date |
|------|---------|------|
| *No spec pages yet* | | |

## Comparisons

| Page | Summary | Date |
|------|---------|------|
| *No comparison pages yet* | | |

## Raw sources available for ingestion

- `research/llm-wiki.md` — schema and operating model for the TinyQuant
  documentation system
- ~~`research/vector-quantization-paper-synthesis.md`~~ — ingested 2026-04-08
  -> [[vector-quantization-paper-synthesis]],
  [[random-preconditioning-without-normalization-overhead]],
  [[two-stage-quantization-and-residual-correction]], [[TurboQuant]],
  [[PolarQuant]], [[QJL]]
- ~~`research/turbo-quant-deep-research-report.md`~~ — ingested 2026-04-08 ->
  [[turbo-quant-deep-research-report]], [[TurboQuant]], [[TinyQuant]]
- ~~`research/turbo-quant-deep-research-report2.md`~~ — ingested 2026-04-08 ->
  [[turbo-quant-code-availability]], [[TurboQuant]], [[TinyQuant]]
- ~~`research/turboquant-integration.md`~~ — ingested 2026-04-08 ->
  [[turboquant-integration]], [[TinyQuant]],
  [[per-collection-compression-policy]], [[storage-codec-architecture]]
- ~~`research/tinyquant-research/README.md`~~ — ingested 2026-04-08 ->
  [[tinyquant-library-research]], [[TinyQuant]],
  [[storage-codec-vs-search-backend-separation]],
  [[storage-codec-architecture]]
- ~~`research/tinyquant-research/better-router-integration.md`~~ — ingested
  2026-04-08 -> [[tinyquant-better-router-integration]],
  [[per-collection-compression-policy]],
  [[storage-codec-architecture]]
