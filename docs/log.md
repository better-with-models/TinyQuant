---
title: Wiki Log
tags:
  - meta
  - log
date-created: 2026-04-08
status: active
---

# Log

> [!info] Operation history
> Append-only record of documentation-system changes. Use the format
> `## [YYYY-MM-DD] operation | description`.

## [2026-04-08] init | Documentation system scaffolding created

Initialized the TinyQuant documentation system using the same structural model
as TurboSwede, without carrying over TurboSwede-specific content:

- Added root `AGENTS.md` to define the wiki schema and operating rules
- Standardized `docs/` as an Obsidian vault with shared settings in
  `.obsidian/app.json`
- Created top-level wiki control pages: [[README]], [[index]], and [[log]]
- Added documentation directories for `entities/`, `concepts/`, `sources/`,
  `comparisons/`, `behavior/`, `design/`, `specs/`, and `assets/`
- Added placeholder structure for `design/domain-layer/` and `specs/plans/`
- Preserved `research/` as the immutable raw-source area

## [2026-04-08] ingest | TinyQuant research seed

Ingested the first TinyQuant-specific research materials from
`docs/research/tinyquant-research/` into the wiki:

- Created [[tinyquant-library-research]] as a source summary for the library
  research seed
- Created [[tinyquant-better-router-integration]] as a source summary for the
  better-router and TurboSwede integration plan
- Created [[TinyQuant]] as the first project entity page
- Created [[storage-codec-vs-search-backend-separation]] and
  [[per-collection-compression-policy]] as core concept pages
- Created [[storage-codec-architecture]] as the first project design page
- Updated [[index]] to register all new source, entity, concept, and design
  pages

The research establishes TinyQuant as a CPU-only, clean-room embedding storage
codec with decompression-first search integration and per-collection rollout
policies for downstream systems.

## [2026-04-08] ingest | TurboQuant research pack

Ingested the broader TurboQuant-related research set from `docs/research/`:

- Created source summaries for [[vector-quantization-paper-synthesis]],
  [[turbo-quant-deep-research-report]],
  [[turbo-quant-code-availability]], and [[turboquant-integration]]
- Added upstream method entity pages for [[TurboQuant]], [[PolarQuant]], and
  [[QJL]]
- Added concept pages for
  [[random-preconditioning-without-normalization-overhead]] and
  [[two-stage-quantization-and-residual-correction]]
- Updated [[TinyQuant]] and [[storage-codec-architecture]] so the project is
  anchored in the full upstream research lineage rather than only the local
  TinyQuant research seed
- Updated [[index]] to register the additional sources, entities, and concepts

The wiki now captures both the local TinyQuant adaptation story and the broader
research lineage that motivates its architecture, provenance discipline, and
deployment boundaries.

## [2026-04-08] author | Domain layer design (DDD)

Created the domain-layer design document set under `docs/design/domain-layer/`
using domain-driven design analysis:

- Updated [[design/domain-layer/README|Domain Layer README]] from placeholder
  to active reading-order hub
- Created [[design/domain-layer/ubiquitous-language|Ubiquitous Language]] with
  canonical term definitions and overloaded-term disambiguation
- Created [[design/domain-layer/context-map|Context Map]] identifying three
  bounded contexts (Codec, Corpus, Backend Protocol) with subdomain
  classification and integration styles
- Created [[design/domain-layer/aggregates-and-entities|Aggregates and
  Entities]] with Corpus as aggregate root, CodecConfig and CompressedVector as
  value objects, Codec as domain service, and invariant documentation
- Created [[design/domain-layer/domain-events|Domain Events]] defining
  business-meaningful events (CodebookTrained, CorpusCreated, VectorsInserted,
  CorpusDecompressed, CompressionPolicyViolationDetected)
- Updated [[index]] to register all new domain-layer pages

Design is deliberately lightweight: module boundaries inside a single Python
library, not microservice decomposition.

## [2026-04-08] author | Behavior layer specifications (BDD)

Created the behavior-layer specification set under
`docs/design/behavior-layer/` using behavior-driven development:

- Created [[design/behavior-layer/README|Behavior Layer README]] as a hub with
  reading order and automation-level guidance
- Created [[design/behavior-layer/codec-compression|Codec Compression
  Behavior]] with scenarios for determinism, index range, residual presence,
  dimension enforcement, and batch operations
- Created [[design/behavior-layer/codec-decompression|Codec Decompression
  Behavior]] with scenarios for FP32 output, config safety, round-trip
  fidelity, and residual correction benefit
- Created [[design/behavior-layer/corpus-management|Corpus Management
  Behavior]] with scenarios for config freezing, policy enforcement, policy
  immutability, and cross-config rejection
- Created [[design/behavior-layer/score-fidelity|Score Fidelity Behavior]]
  with quality contract scenarios for Pearson rho, rank preservation, bit-width
  ordering, and corpus-size independence
- Created [[design/behavior-layer/backend-protocol|Backend Protocol Behavior]]
  with integration boundary scenarios for FP32-only handoff, adapter
  translation, error isolation, and query passthrough
- Updated [[index]] to register all new behavior-layer pages

All scenarios are downstream-agnostic and target Python API-level automation.
Scrubbed better-router/TurboSwede/OpenViking references from both document
sets to keep TinyQuant's spec portable.

## [2026-04-08] author | Architecture design considerations

Created the architecture design consideration set under
`docs/design/architecture/` as binding implementation policies:

- Created [[design/architecture/README|Architecture Design Considerations]]
  hub with reading order and relationship diagram
- Created [[design/architecture/test-driven-development|Test-Driven
  Development]] establishing red-green-refactor as mandatory workflow with
  test levels, design signals, and pytest/hypothesis tooling
- Created [[design/architecture/solid-principles|SOLID Principles]] mapping
  each principle to TinyQuant's codec, corpus, and backend structure with
  concrete violation examples
- Created [[design/architecture/file-and-complexity-policy|File and Complexity
  Policy]] enforcing one public class per file and CC max 7 per function
- Created [[design/architecture/high-coherence|High Coherence]] defining four
  coherence levels and their enforcement through package structure
- Created [[design/architecture/low-coupling|Low Coupling]] specifying acyclic
  dependency direction, value objects at boundaries, and measurement signals
- Created [[design/architecture/linting-and-tooling|Linting and Tooling]]
  establishing ruff + mypy + pytest as CI gates with warnings-as-errors and
  justified-suppression-only policy
- Created [[design/architecture/docstring-policy|Docstring Policy]] requiring
  PEP 257 Google-style docstrings on all public symbols with ruff rule D
  enforcement
- Created [[design/architecture/type-safety|Type Safety]] requiring mypy
  strict mode with full annotations and Protocol-based contracts
- Created [[design/architecture/namespace-and-module-structure|Namespace and
  Module Structure]] mapping bounded contexts to Python packages with explicit
  public APIs, acyclic imports, and architecture tests
- Updated [[index]] to register all new architecture pages

All policies are framed as enforceable CI gates, not aspirational guidelines.

## [2026-04-08] author | Class specifications

Created the class specification set under `docs/classes/` indexing all planned
Python class files with members, responsibilities, and relationships:

- Created [[classes/README|Class Specifications]] as the master index organized
  by package (codec, corpus, backend, shared)
- **Codec package (6 pages):**
  [[classes/codec-config|CodecConfig]],
  [[classes/rotation-matrix|RotationMatrix]],
  [[classes/codebook|Codebook]],
  [[classes/compressed-vector|CompressedVector]],
  [[classes/codec|Codec]],
  [[classes/quantize-internal|_quantize (internal)]]
- **Corpus package (4 pages):**
  [[classes/corpus|Corpus]],
  [[classes/vector-entry|VectorEntry]],
  [[classes/compression-policy|CompressionPolicy]],
  [[classes/corpus-events|Corpus Events]]
- **Backend package (4 pages):**
  [[classes/search-backend|SearchBackend]],
  [[classes/search-result|SearchResult]],
  [[classes/brute-force-backend|BruteForceBackend]],
  [[classes/pgvector-adapter|PgvectorAdapter]]
- **Shared (1 page):**
  [[classes/shared-types|Shared Types]]
- Updated [[index]] to register all 16 class specification pages

Each page specifies responsibility, file path, category, typed fields,
method signatures with pre/postconditions, invariants, relationships, and
corresponding test file.
