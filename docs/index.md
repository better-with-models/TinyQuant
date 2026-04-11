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
| [[sources/vector-quantization-paper-synthesis\|vector-quantization-paper-synthesis]] | Cross-paper synthesis of QJL, PolarQuant, and TurboQuant, including the main algorithmic themes relevant to TinyQuant | 2026-04-08 |
| [[sources/turbo-quant-deep-research-report\|turbo-quant-deep-research-report]] | Deep research summary of TurboQuant's claims, deployment posture, and relationship to neighboring quantization methods | 2026-04-08 |
| [[turbo-quant-code-availability]] | Summary of official and community code availability across TurboQuant and related methods, with provenance implications for TinyQuant | 2026-04-08 |
| [[sources/turboquant-integration\|turboquant-integration]] | System-integration summary for bringing TurboQuant-style compression into the better-router stack | 2026-04-08 |

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
| [[design/rust/README\|Rust Port Design Overview]] | Authoritative design entry point for the Rust reimplementation: reading order, layer relationships, authority boundary | 2026-04-10 |
| [[design/rust/goals-and-non-goals\|Rust Port Goals and Non-Goals]] | What "ultra-high performance" means for TinyQuant and where the port deliberately stops | 2026-04-10 |
| [[design/rust/crate-topology\|Crate Topology and Module Structure]] | Cargo workspace layout, crate graph, per-crate responsibilities, and module organization | 2026-04-10 |
| [[design/rust/type-mapping\|Type Mapping from Python]] | One-to-one mapping of every public Python symbol to its Rust counterpart | 2026-04-10 |
| [[design/rust/numerical-semantics\|Numerical Semantics and Determinism]] | Rotation matrix, quantization, residual, fp16 — bit-for-bit parity plan | 2026-04-10 |
| [[design/rust/memory-layout\|Memory Layout and Allocation Strategy]] | SoA vs AoS, alignment, owned vs borrowed buffers, arenas | 2026-04-10 |
| [[design/rust/simd-strategy\|SIMD Strategy]] | Portable SIMD, runtime dispatch, x86 AVX2/AVX-512, aarch64 NEON | 2026-04-10 |
| [[design/rust/parallelism\|Parallelism and Concurrency]] | Rayon, thread pools, Send/Sync contracts | 2026-04-10 |
| [[design/rust/error-model\|Error Model]] | `thiserror`, error taxonomy, panic discipline | 2026-04-10 |
| [[design/rust/serialization-format\|Serialization Format]] | Wire format parity with Python, endian, versioning, zero-copy reads | 2026-04-10 |
| [[design/rust/ffi-and-bindings\|FFI and Bindings]] | C ABI, pyo3 bindings for drop-in replacement, WASM feature | 2026-04-10 |
| [[design/rust/benchmark-harness\|Benchmark Harness and Performance Budgets]] | Criterion, flamegraph, regression gates | 2026-04-10 |
| [[design/rust/testing-strategy\|Testing Strategy]] | Unit, property, snapshot, cross-language parity, fuzz | 2026-04-10 |
| [[design/rust/ci-cd\|CI/CD for the Rust Crate]] | GitHub Actions matrix, clippy, miri, cargo-deny, OIDC release | 2026-04-10 |
| [[design/rust/feature-flags\|Feature Flags and Optional Dependencies]] | `default`, `pgvector`, `pyo3`, `simd`, `std` feature flags | 2026-04-10 |
| [[design/rust/release-strategy\|Release and Versioning]] | crates.io, PyPI via maturin, MSRV policy | 2026-04-10 |
| [[design/rust/risks-and-mitigations\|Risks and Mitigations]] | LAPACK determinism, QR sign conventions, fp16 round-trips, cross-platform parity | 2026-04-10 |

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

## Quality Assurance

| Page | Summary | Date |
|------|---------|------|
| [[qa/README|Quality Assurance]] | QA hub: test pyramid, tooling, relationship to design docs | 2026-04-08 |
| [[qa/unit-tests/README|Unit Tests]] | Test specs for all planned classes; 10 per-class test spec pages (~150 tests total) | 2026-04-08 |
| [[qa/unit-tests/test-codec-config|CodecConfig tests]] | 15 tests: construction, immutability, properties, equality, property-based | 2026-04-08 |
| [[qa/unit-tests/test-rotation-matrix|RotationMatrix tests]] | 12 tests: factory, orthogonality, apply/inverse, property-based | 2026-04-08 |
| [[qa/unit-tests/test-codebook|Codebook tests]] | 14 tests: training, quantization, dequantization, immutability | 2026-04-08 |
| [[qa/unit-tests/test-compressed-vector|CompressedVector tests]] | 12 tests: construction, properties, serialization round-trip | 2026-04-08 |
| [[qa/unit-tests/test-codec|Codec tests]] | 25 tests: compress, decompress, round-trip, batch, build, property-based | 2026-04-08 |
| [[qa/unit-tests/test-corpus|Corpus tests]] | 30 tests: construction, insert, batch, policy, get/contains, decompress, remove | 2026-04-08 |
| [[qa/unit-tests/test-vector-entry|VectorEntry tests]] | 8 tests: construction, delegation, equality, metadata | 2026-04-08 |
| [[qa/unit-tests/test-compression-policy|CompressionPolicy tests]] | 6 tests: requires_codec, storage_dtype for each variant | 2026-04-08 |
| [[qa/unit-tests/test-events|Corpus Events tests]] | 10 tests: field correctness and immutability for all 4 event types | 2026-04-08 |
| [[qa/unit-tests/test-brute-force|BruteForceBackend tests]] | 12 tests: ingest, search ranking, remove, clear | 2026-04-08 |
| [[qa/e2e-tests/README|End-to-End Tests]] | 10 full-pipeline scenarios: compress→store→decompress→search, fidelity, determinism | 2026-04-08 |
| [[qa/integration-plan/README|Integration Plan]] | Cross-boundary test strategy: 2 internal + 3 external boundary test suites | 2026-04-08 |
| [[qa/verification-plan/README|Verification Plan]] | "Built it right": 8 verification dimensions covering architecture, types, lint, coverage, invariants | 2026-04-08 |
| [[qa/validation-plan/README|Validation Plan]] | "Built the right thing": 7 validation dimensions covering fidelity, compression ratio, research alignment | 2026-04-08 |

## Behavior Specifications

| Page | Summary | Date |
|------|---------|------|
| [[behavior/README|Behavior README]] | Placeholder for future behavior specs and acceptance-criteria organization | 2026-04-08 |

## CI/CD Plans

| Page | Summary | Date |
|------|---------|------|
| [[CI-plan/README|CI Plan]] | Continuous integration strategy: fail-fast pipeline, GitHub Actions, quality gates | 2026-04-08 |
| [[CI-plan/pipeline-stages|Pipeline Stages]] | 6-stage fail-fast ordering: lint, typecheck, build, tests, coverage, artifact | 2026-04-08 |
| [[CI-plan/workflow-definition|Workflow Definition]] | Full `.github/workflows/ci.yml` specification with job graph and matrix strategy | 2026-04-08 |
| [[CI-plan/quality-gates|Quality Gates]] | Hard gates (block merge), soft gates (advisory), release gates (pre-publish) | 2026-04-08 |
| [[CD-plan/README|CD Plan]] | Continuous delivery strategy: tag-triggered release, PyPI trusted publishing | 2026-04-08 |
| [[CD-plan/release-workflow|Release Workflow]] | Full `.github/workflows/release.yml` specification with OIDC and calibration gates | 2026-04-08 |
| [[CD-plan/artifact-management|Artifact Management]] | Build once/promote, immutability rules, tagging convention, verification steps | 2026-04-08 |
| [[CD-plan/versioning-and-changelog|Versioning and Changelog]] | Semantic versioning rules, breaking change definitions, changelog format | 2026-04-08 |

## Roadmap and Plans

| Page | Summary | Date |
|------|---------|------|
| [[roadmap|Implementation Roadmap]] | 10-phase implementation plan with dependency graph, each phase scoped for one AI agent turn | 2026-04-08 |
| [[plans/phase-01-scaffolding|Phase 1: Scaffolding]] | pyproject.toml, package structure, tooling config, smoke test | 2026-04-08 |
| [[plans/phase-02-codec-value-objects|Phase 2: Codec Value Objects]] | CodecConfig, RotationMatrix, Codebook, CompressedVector + ~53 tests | 2026-04-08 |
| [[plans/phase-03-codec-service|Phase 3: Codec Service]] | _quantize helpers, Codec class, compress/decompress + ~25 tests | 2026-04-08 |
| [[plans/phase-04-corpus-layer|Phase 4: Corpus Layer]] | CompressionPolicy, VectorEntry, events, Corpus aggregate + ~54 tests | 2026-04-08 |
| [[plans/phase-05-backend-layer|Phase 5: Backend Layer]] | SearchBackend protocol, BruteForceBackend + ~12 tests | 2026-04-08 |
| [[plans/phase-06-serialization|Phase 6: Serialization]] | CompressedVector to_bytes/from_bytes, format versioning + ~11 tests | 2026-04-08 |
| [[plans/phase-07-architecture-e2e-tests|Phase 7: Architecture & E2E Tests]] | Dependency direction tests, integration tests, 8 E2E scenarios | 2026-04-08 |
| [[plans/phase-08-ci-cd-workflows|Phase 8: CI/CD Workflows]] | ci.yml and release.yml GitHub Actions workflows | 2026-04-08 |
| [[plans/phase-09-pgvector-adapter|Phase 9: Pgvector Adapter]] | PgvectorAdapter ACL + ~6 integration tests | 2026-04-08 |
| [[plans/phase-10-calibration-release|Phase 10: Calibration & Release]] | Calibration tests, CHANGELOG, README, v0.1.0 tag | 2026-04-08 |
| [[plans/rust/phase-11-rust-workspace-scaffold\|Phase 11: Rust Workspace Scaffold]] | Cargo workspace scaffold: 10 members, toolchain pin 1.78.0, lint wall, smoke tests, rust-ci.yml | 2026-04-10 |
| [[plans/rust/phase-12-shared-types-and-errors\|Phase 12: Shared Types and Errors]] | `tinyquant-core::types` aliases and `tinyquant-core::errors` enums, MSRV 1.78→1.81, thiserror v2 under no_std — **complete** | 2026-04-10 |
| [[plans/rust/phase-13-rotation-numerics\|Phase 13: Rotation Matrix and Numerics]] | `codec::CodecConfig`, `ChaChaGaussianStream`, `RotationMatrix`, `RotationCache`, Python-parity `config_hash`, Rust-canonical rotation fixtures under LFS — **complete** | 2026-04-10 |
| [[design/rust/phase-13-implementation-notes\|Phase 13 Implementation Notes]] | Execution-log view of Phase 13: what landed, deviations from the plan, gotchas (Python bool parity, faer stack, clippy lints), locked-in invariants, and carryover into Phase 14+ | 2026-04-10 |
| [[plans/rust/phase-14-codebook-quantize\|Phase 14: Codebook and Quantize Kernels]] | `tinyquant_core::codec::Codebook` value object + private `scalar_quantize` / `scalar_dequantize` kernels, Python-parity `train` + `quantize` across `bw ∈ {2, 4, 8}`, 10 000 × 64 training corpus + 10 000-value quantize corpus frozen under LFS — **complete** | 2026-04-10 |
| [[design/rust/phase-14-implementation-notes\|Phase 14 Implementation Notes]] | Execution-log view of Phase 14: bit-width sweep scoped in during execution, proptest deferred on MSRV 1.81 (replaced by a deterministic `rand_chacha` scan), runtime `fs::read` fixture pattern, narrow `#[allow(clippy::cast_*)]` on `Codebook::train`, and the Python-literal tie-break gotcha in the round-trip test | 2026-04-10 |
| [[design/rust/phase-15-implementation-notes\|Phase 15 Implementation Notes]] | Execution-log view of Phase 15: codec service, residual, and CompressedVector — Rust-canonical fixture strategy due to RNG divergence, clippy constraint patterns, and fidelity gate numbers | 2026-04-10 |
| [[design/rust/phase-17-implementation-notes\|Phase 17 Implementation Notes]] | Execution-log view of Phase 17: `tinyquant-io` zero-copy views, Level-2 TQCV corpus file container, and mmap-based reader — TQCV magic-byte layout, `mmap-lock` feature flag, and `MmapView` lifetime safety model | 2026-04-11 |
| [[design/rust/phase-18-implementation-notes\|Phase 18 Implementation Notes]] | Execution-log view of Phase 18: `Corpus` aggregate root, vector insertion, batch atomicity, three-policy decompression, domain events, and insertion-ordered vector map | 2026-04-11 |
| [[plans/rust/phase-15-codec-service-residual\|Phase 15: Codec Service, Residual, and CompressedVector]] | `tinyquant-core::codec::Codec` service with compress/decompress, `compute_residual`, `CompressedVector` — Rust-canonical fixture strategy | 2026-04-10 |
| [[plans/rust/phase-16-serialization-parity\|Phase 16: Serialization and Python Byte Parity]] | `tinyquant-io` serialization with Python byte-level parity, endian contract, version tag | 2026-04-10 |
| [[plans/rust/phase-17-zero-copy-mmap\|Phase 17: Zero-copy Views and Mmap Corpus Files]] | `tinyquant-io` zero-copy views, Level-2 TQCV file container, mmap-based reader | 2026-04-10 |
| [[plans/rust/phase-18-corpus-aggregate\|Phase 18: Corpus Aggregate and Domain Events]] | `tinyquant-core::corpus::Corpus` aggregate root with insertion, policy enforcement, domain events | 2026-04-10 |
| [[plans/rust/phase-19-brute-force-pgvector\|Phase 19: Brute-force Backend and Pgvector Adapter]] | `tinyquant-bruteforce` exhaustive search backend and `tinyquant-pgvector` ACL adapter | 2026-04-10 |
| [[plans/rust/phase-20-simd-kernels\|Phase 20: SIMD Kernels and Runtime Dispatch]] | AVX2/AVX-512/NEON SIMD kernels with is_x86_feature_detected! runtime dispatch | 2026-04-10 |
| [[plans/rust/phase-21-rayon-batch-benches\|Phase 21: Rayon Batch Paths, Calibration, and Benchmarks]] | Rayon parallel batch paths, criterion benchmarks, performance regression gates | 2026-04-10 |
| [[plans/rust/phase-22-pyo3-cabi-release\|Phase 22: Pyo3, C ABI, and Release]] | pyo3 bindings, C ABI via `tinyquant-sys`, standalone CLI binary, multi-arch release | 2026-04-10 |

## Specs

| Page | Summary | Date |
|------|---------|------|
| [[specs/plans/2026-04-09-github-actions-node24-upgrade\|GitHub Actions Node 24 Upgrade]] | Plan: bump artifact actions to `@v5`, replace `softprops/action-gh-release` with `gh release create`, reconcile CI/CD wiki | 2026-04-09 |
| [[superpowers/plans/2026-04-08-phase-05-backend-layer\|Phase 5 Execution Plan]] | Agentic task checklist for Phase 5: SearchBackend protocol, SearchResult value object, and BruteForceBackend reference implementation | 2026-04-08 |
| [[superpowers/plans/2026-04-09-readme-gfm-refresh\|README GFM Refresh Plan]] | Agentic task checklist for the GitHub-flavored Markdown README refresh: rich visual vocabulary, canonical tagline, and cross-file prose alignment rule | 2026-04-09 |
| [[superpowers/plans/2026-04-10-phase-12-shared-types-and-errors\|Phase 12 Execution Plan]] | Agentic task checklist for Phase 12: shared types and error enums in tinyquant-core, MSRV 1.81, thiserror v2 under no_std | 2026-04-10 |

## Comparisons

| Page | Summary | Date |
|------|---------|------|
| *No comparison pages yet* | | |

## Meta

| Page | Summary | Date |
|------|---------|------|
| [[README\|Wiki README]] | Human-facing overview of the vault structure, Obsidian conventions, operating procedures, and the three-layer model | 2026-04-08 |
| [[log\|Change Log]] | Append-only operational history for scaffolding, ingests, and maintenance work | 2026-04-08 |

## Raw sources available for ingestion

- [[research/llm-wiki]] — schema and operating model for the TinyQuant
  documentation system
- ~~[[research/vector-quantization-paper-synthesis]]~~ — ingested 2026-04-08
  -> [[sources/vector-quantization-paper-synthesis\|vector-quantization-paper-synthesis]],
  [[random-preconditioning-without-normalization-overhead]],
  [[two-stage-quantization-and-residual-correction]], [[TurboQuant]],
  [[PolarQuant]], [[QJL]]
- ~~[[research/turbo-quant-deep-research-report]]~~ — ingested 2026-04-08 ->
  [[sources/turbo-quant-deep-research-report\|turbo-quant-deep-research-report]], [[TurboQuant]], [[TinyQuant]]
- ~~[[research/turbo-quant-deep-research-report2]]~~ — ingested 2026-04-08 ->
  [[turbo-quant-code-availability]], [[TurboQuant]], [[TinyQuant]]
- ~~[[research/turboquant-integration]]~~ — ingested 2026-04-08 ->
  [[sources/turboquant-integration\|turboquant-integration]], [[TinyQuant]],
  [[per-collection-compression-policy]], [[storage-codec-architecture]]
- ~~[[research/tinyquant-research/README]]~~ — ingested 2026-04-08 ->
  [[tinyquant-library-research]], [[TinyQuant]],
  [[storage-codec-vs-search-backend-separation]],
  [[storage-codec-architecture]]
- ~~[[research/tinyquant-research/better-router-integration]]~~ — ingested
  2026-04-08 -> [[tinyquant-better-router-integration]],
  [[per-collection-compression-policy]],
  [[storage-codec-architecture]]
