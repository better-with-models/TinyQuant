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

- Created source summaries for [[sources/vector-quantization-paper-synthesis\|vector-quantization-paper-synthesis]],
  [[sources/turbo-quant-deep-research-report\|turbo-quant-deep-research-report]],
  [[turbo-quant-code-availability]], and [[sources/turboquant-integration\|turboquant-integration]]
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

## [2026-04-08] author | Quality assurance documentation

Created the QA documentation set under `docs/qa/` with five subdirectories:

- Created [[qa/README|Quality Assurance]] hub with test pyramid, tooling
  summary, and cross-references to design documentation
- **Unit tests (11 pages):**
  [[qa/unit-tests/README|index]] plus 10 per-class test specs covering
  [[qa/unit-tests/test-codec-config|CodecConfig]],
  [[qa/unit-tests/test-rotation-matrix|RotationMatrix]],
  [[qa/unit-tests/test-codebook|Codebook]],
  [[qa/unit-tests/test-compressed-vector|CompressedVector]],
  [[qa/unit-tests/test-codec|Codec]],
  [[qa/unit-tests/test-corpus|Corpus]],
  [[qa/unit-tests/test-vector-entry|VectorEntry]],
  [[qa/unit-tests/test-compression-policy|CompressionPolicy]],
  [[qa/unit-tests/test-events|Corpus Events]],
  [[qa/unit-tests/test-brute-force|BruteForceBackend]]
  (~150 test cases total)
- **E2E tests (1 page):**
  [[qa/e2e-tests/README|10 full-pipeline scenarios]] covering
  compress→store→decompress→search, passthrough, FP16, cross-config rejection,
  fidelity, serialization, pgvector, empty corpus, residual comparison, and
  cross-run determinism
- **Integration plan (1 page):**
  [[qa/integration-plan/README|2 internal + 3 external boundary test suites]]
  covering codec→corpus handoff, corpus→backend handoff, serialization,
  pgvector adapter, and future persistence
- **Verification plan (1 page):**
  [[qa/verification-plan/README|8 verification dimensions]] covering
  architecture conformance, type safety, linting, complexity, docstrings,
  coverage, invariant preservation, and serialization compatibility
- **Validation plan (1 page):**
  [[qa/validation-plan/README|7 validation dimensions]] covering score
  fidelity, compression ratio, determinism, research baseline alignment,
  API usability, policy correctness, and acceptance criteria traceability
- Updated [[index]] to register all 16 QA pages

## [2026-04-08] author | CI/CD plans

Created CI and CD plan documentation under `docs/CI-plan/` and `docs/CD-plan/`:

- **CI Plan (4 pages):**
  [[CI-plan/README|hub]] with principles and reading order,
  [[CI-plan/pipeline-stages|Pipeline Stages]] with 6-stage fail-fast ordering
  (lint→typecheck→build→tests→coverage→artifact) and Mermaid job graph,
  [[CI-plan/workflow-definition|Workflow Definition]] specifying the full
  `.github/workflows/ci.yml` with triggers, concurrency, caching, matrix
  strategy (Python 3.12/3.13), and all job YAML,
  [[CI-plan/quality-gates|Quality Gates]] classifying 14 hard gates, 4 soft
  gates, and 5 release gates with escalation policy
- **CD Plan (4 pages):**
  [[CD-plan/README|hub]] with delivery model diagram and environment
  definitions,
  [[CD-plan/release-workflow|Release Workflow]] specifying the full
  `.github/workflows/release.yml` with tag verification, calibration tests,
  TestPyPI→PyPI promotion via OIDC trusted publishing and manual approval gate,
  [[CD-plan/artifact-management|Artifact Management]] with build-once/promote
  rules, immutability policy, and retention schedule,
  [[CD-plan/versioning-and-changelog|Versioning and Changelog]] with semantic
  versioning rules, breaking change classification, and changelog format
- Updated [[index]] to register all 8 CI/CD pages

## [2026-04-08] author | Implementation roadmap and phase plans

Created the implementation roadmap and 10 detailed phase plans:

- Created [[roadmap|Implementation Roadmap]] with 10-phase overview, Gantt-style
  Mermaid diagram, dependency chain, per-phase design constraints, and
  completion criteria
- Created 10 phase plan documents under `docs/plans/`:
  1. [[plans/phase-01-scaffolding|Project Scaffolding]] — pyproject.toml, package
     dirs, tooling config, smoke test (~19 files, ~150 LOC config)
  2. [[plans/phase-02-codec-value-objects|Codec Value Objects]] — CodecConfig,
     RotationMatrix, Codebook, CompressedVector, _types (~53 tests, ~1100 LOC)
  3. [[plans/phase-03-codec-service|Codec Service]] — _quantize helpers, Codec
     class, batch ops, module functions (~25 tests, ~700 LOC)
  4. [[plans/phase-04-corpus-layer|Corpus Layer]] — CompressionPolicy,
     VectorEntry, events, Corpus aggregate (~54 tests, ~1200 LOC)
  5. [[plans/phase-05-backend-layer|Backend Layer]] — SearchBackend protocol,
     SearchResult, BruteForceBackend (~12 tests, ~350 LOC)
  6. [[plans/phase-06-serialization|Serialization]] — CompressedVector binary
     format with versioning (~11 tests, ~350 LOC)
  7. [[plans/phase-07-architecture-e2e-tests|Architecture & E2E Tests]] —
     dependency direction, cross-boundary integration, 8 E2E scenarios (~21
     tests, ~600 LOC)
  8. [[plans/phase-08-ci-cd-workflows|CI/CD Workflows]] — ci.yml and
     release.yml GitHub Actions (~300 LOC YAML)
  9. [[plans/phase-09-pgvector-adapter|Pgvector Adapter]] — PgvectorAdapter ACL
     with parameterized SQL (~6 tests, ~300 LOC)
  10. [[plans/phase-10-calibration-release|Calibration & Release]] — fidelity,
      compression ratio, determinism tests; CHANGELOG; README; v0.1.0 tag (~16
      tests, ~500 LOC)
- Updated [[index]] to register all 11 roadmap/plan pages

Each phase is scoped for one AI agent working turn, follows TDD
red-green-refactor, and leaves the repo in a working state if interrupted.

## [2026-04-08] implement | Phase 1: Project Scaffolding

Implemented the Python project structure and tooling configuration:

- Created `pyproject.toml` with hatchling build, ruff, mypy, and pytest config
- Created package skeleton: `src/tinyquant_cpu/`, `src/tinyquant_cpu/codec/`,
  `src/tinyquant_cpu/corpus/`, `src/tinyquant_cpu/backend/`
- Created test skeleton: `tests/`, `tests/codec/`, `tests/corpus/`,
  `tests/backend/`, `tests/integration/`, `tests/e2e/`, `tests/architecture/`,
  `tests/calibration/`
- Added `_types.py` with shared type aliases
- Added 1 smoke test (`test_import_tinyquant`)
- All tooling passes: ruff, mypy --strict, pytest

## [2026-04-08] implement | Phase 2: Codec Value Objects

Implemented the four codec value objects with full TDD:

- `CodecConfig` — frozen dataclass with validation, config_hash, bit_width
  support for {2, 4, 8}
- `RotationMatrix` — QR-decomposed orthogonal matrix with Haar measure sign
  correction, apply/apply_inverse
- `Codebook` — quantile-trained lookup table with quantize/dequantize
  (searchsorted + nearest neighbor)
- `CompressedVector` — frozen output container with indices, residual,
  config_hash; serialization stubs for Phase 6
- 54 tests (including hypothesis property-based), 97% codec coverage

## [2026-04-08] implement | Phase 3: Codec Service

Implemented the stateless Codec domain service wiring Phase 2 value objects:

- `_errors.py` — DimensionMismatchError, ConfigMismatchError,
  CodebookIncompatibleError
- `_quantize.py` — scalar_quantize, scalar_dequantize, compute_residual
  (FP16 projection), apply_residual
- `Codec` class — compress, decompress, compress_batch, decompress_batch,
  build_codebook, build_rotation
- Module-level `compress()` and `decompress()` convenience functions
- 31 tests (including 2 hypothesis), 97% codec coverage, 86 total tests

## [2026-04-08] implement | Phase 4: Corpus Layer

Implemented the corpus bounded context on top of the codec service:

- `CompressionPolicy` enum — COMPRESS, PASSTHROUGH, FP16 with requires_codec
  and storage_dtype
- `VectorEntry` entity — identity equality by vector_id, delegation properties
- 4 domain events — CorpusCreated, VectorsInserted, CorpusDecompressed,
  CompressionPolicyViolationDetected (all frozen dataclasses)
- `Corpus` aggregate root — insert, insert_batch (atomic), get, contains,
  decompress, decompress_all, remove, pending_events
- `DuplicateVectorError` added to codec error types
- PASSTHROUGH/FP16 policies wrap raw data in CompressedVector for type
  uniformity
- 59 tests, 97% corpus coverage, 145 total tests passing

## [2026-04-08] implement | Phase 5: Backend Layer

Implemented the backend protocol bounded context on top of the corpus layer:

- `SearchResult` frozen dataclass — immutable value object with descending
  `__lt__` ordering for ranked results
- `SearchBackend` protocol — structural typing contract (search, ingest, remove)
  enforcing FP32-only vector boundary
- `BruteForceBackend` reference implementation — exhaustive cosine similarity
  search with in-memory dict storage, ingest, remove, clear, and count property
- All imports under `TYPE_CHECKING` where not needed at runtime (numpy, NDArray,
  Sequence) matching project conventions
- Public re-exports from `tinyquant_cpu.backend.__init__`
- 16 tests (3 SearchResult, 3 ingest, 7 search, 2 remove, 1 clear), 96%
  backend coverage, 161 total tests passing

## [2026-04-08] implement | Phase 6: Serialization

Implemented CompressedVector binary serialization with format versioning:

- `to_bytes()` and `from_bytes()` with version-tagged binary format
- 11 tests covering round-trip, format validation, and error cases
- 172 total tests passing

## [2026-04-08] implement | Phase 7: Architecture & E2E Tests

Implemented architecture conformance and end-to-end test suites:

- Architecture tests: dependency direction, module boundary enforcement
- Integration tests: codec→corpus handoff, corpus→backend handoff
- E2E tests: 8 full-pipeline scenarios (compress→store→decompress→search)
- 23 new tests, 193 total tests passing

## [2026-04-08] implement | Phase 8: CI/CD Workflows

Created GitHub Actions workflow files matching the specifications in
[[CI-plan/workflow-definition|Workflow Definition]] and
[[CD-plan/release-workflow|Release Workflow]]:

- `.github/workflows/ci.yml` — 5-job CI pipeline: lint, markdown-lint,
  typecheck, test (matrix 3.12/3.13), build-artifact with concurrency
  cancellation, pip/mypy caching, and coverage upload
- `.github/workflows/release.yml` — 5-job release pipeline: verify-tag,
  calibration-tests, publish-testpypi (OIDC), publish-pypi (manual approval),
  github-release with auto-generated notes
- `.github/release-drafter.yml` — auto-labeling and categorized release notes
- `.pre-commit-config.yaml` already matched spec; no changes needed
- Updated [[roadmap]] to mark Phase 8 complete

## [2026-04-09] maint | GitHub Actions Node 24 upgrade

Addressed the "Node.js 20 actions are deprecated" warnings surfaced by the
release workflow runs (GitHub force-upgrades JS actions to Node 24 on
2026-06-02):

- Bumped `actions/upload-artifact` and `actions/download-artifact` from `@v4`
  to `@v5` (first Node-24 major) across `.github/workflows/release.yml` and
  `.github/workflows/ci.yml`
- Replaced `softprops/action-gh-release@v2` with an inline `gh release create`
  shell step in the `github-release` job. Upstream has no Node 24 release
  as of this date (softprops/action-gh-release#742; PRs #670 and #774 still
  open), so the replacement removes the blocking third-party dependency.
  The new step consumes the `release-notes.md` file already produced by the
  preceding `Generate release notes file` step via `--notes-file`, and adds
  a nullglob guard so an empty `dist/` fails loudly instead of passing a
  literal `dist/*` argument to `gh`.
- Reconciled [[CD-plan/release-workflow|Release Workflow]],
  [[CI-plan/workflow-definition|Workflow Definition]], and
  [[CD-plan/versioning-and-changelog|Versioning and Changelog]] to reflect
  the new action versions and the replaced release step
- Plan recorded at [[specs/plans/2026-04-09-github-actions-node24-upgrade|GitHub Actions Node 24 Upgrade]]

## [2026-04-10] scaffold | Phase 11 — Rust workspace scaffold

Created the Cargo workspace under `rust/` with all 10 workspace members.

- `rust/Cargo.toml` — workspace root, resolver v2, 10 members, shared deps
- `rust/rust-toolchain.toml` — channel 1.78.0, rustfmt + clippy + thumbv7em target
- `rust/.cargo/config.toml` — RUSTFLAGS = -D warnings, cargo xtask alias
- `rust/crates/tinyquant-{core,io,bruteforce,pgvector,sys,py,cli,bench,fuzz}/`
- `rust/xtask/` — cargo xtask dispatcher (fmt/lint/test/help)
- `.github/workflows/rust-ci.yml` — fmt → clippy → {test, no-std-check}
- All 5 acceptance criteria pass: build, test, fmt, clippy, no_std cross-compile

## [2026-04-10] implement | Phase 12 — Shared Types and Error Enums

Populated `tinyquant-core::types` (6 type aliases: `VectorId`, `ConfigHash`,
`CorpusId`, `Vector`, `VectorSlice`, `Timestamp`) and `tinyquant-core::errors`
(`CodecError` 11 variants, `CorpusError` 4 variants with `#[from] #[source]`
source chain, `BackendError` 3 variants). Bumped MSRV 1.78 → 1.81 and
thiserror 1 → 2 to enable `no_std` error derives via `core::error::Error`.
28 `tinyquant-core` tests pass (smoke + types + errors); `no_std` build on
`thumbv7em-none-eabihf` verified. Clippy and fmt clean.

## [2026-04-10] implement | Phase 13 — Rotation Matrix and Numerical Foundations

Added the `tinyquant-core::codec` module with `CodecConfig` (Python-parity
SHA-256 `config_hash`), `ChaChaGaussianStream` (seedable ChaCha20 +
Box-Muller f64 stream), `RotationMatrix` (faer QR + Haar sign correction,
row-major `Arc<[f64]>`, f32↔f64 `apply_into` / `apply_inverse_into`), and
`RotationCache` (8-entry spin-mutex LRU, mirrors Python
`functools.lru_cache(maxsize=8)`). Wired `faer`, `rand_chacha`, `sha2`,
`hex`, `spin`, `libm`, and `serde_json` (dev) into the core crate; added
`scripts/generate_rust_fixtures.py` to emit `config_hashes.json` (120
canonical triples) from the Python reference and an `examples/
dump_rotation_fixture.rs` binary to freeze rotation f64 snapshots under
`tests/fixtures/rotation/` (LFS-tracked). New xtask subcommands:
`cargo xtask fixtures refresh-hashes | refresh-rotation | refresh-all`.

Parity verified:
- 13 `codec_config` tests, including a 120-triple sweep of
  `(bit_width, seed, dimension, residual_enabled)` against the
  Python-generated `config_hashes.json`.
- 8 `rotation_matrix` tests (shape, orthogonality within `1e-12`,
  deterministic build, round-trip `< 1e-5`, `LengthMismatch` rejects).
- 3 `rotation_fixture_parity` tests verifying bit-for-bit equality
  against the frozen `seed_42_dim_64` and `seed_42_dim_768` f64 dumps.
- 8 `rotation_cache` tests (LRU eviction, MRU promotion, capacity=0
  bypass).

Workspace cargo check, clippy (`-D warnings` across the pedantic +
nursery groups), `cargo xtask test`, `cargo build -p tinyquant-core
--no-default-features`, and the embedded
`--target thumbv7em-none-eabihf --no-default-features` build are all
green. Fixture reproducibility confirmed by `md5sum` before and after a
fresh `cargo xtask fixtures refresh-all`.

Deferred by design (not in Phase 13):
- legacy-Python (PCG64) vs Rust-canonical cosine parity — Phase 15+
- pyo3 bridge that lets Python consumers opt into the canonical path —
  Phase 15+

## [2026-04-10] phase-14 | Codebook value object + scalar quantize kernels landed

Shipped [[plans/rust/phase-14-codebook-quantize|Phase 14]] on a
dedicated `phase-14-codebook-quantize` feature branch, bringing
`tinyquant_core::codec::Codebook` online together with the private
`scalar_quantize` / `scalar_dequantize` reference kernels that Phase 20
will benchmark its SIMD path against. All training, quantize, and
dequantize math is byte-identical to
`src/tinyquant_cpu/codec/codebook.py` across every supported bit
width.

New Rust source modules (no_std, lint-clean under the crate's
`clippy::pedantic + nursery + unwrap_used + indexing_slicing` profile):
- `rust/crates/tinyquant-core/src/codec/codebook.rs` —
  `Codebook { entries: Arc<[f32]>, bit_width }` with `new`, `train`
  (f64 flatten + `f64::total_cmp` sort + linear-interpolated quantiles
  + `as f32` cast + defensive re-sort + explicit
  `InsufficientTrainingData` gate), `num_entries`, `bit_width`,
  `entries`, `quantize_into`, `dequantize_into`, and convenience
  `quantize` / `dequantize` allocators. `PartialEq` compares entry
  bits, not allocation identity.
- `rust/crates/tinyquant-core/src/codec/quantize.rs` — panic-free
  `scalar_quantize` (linear scan matching
  `np.searchsorted(side="left")` + strict-`<` tie-break on
  `left_dist < right_dist`) and `scalar_dequantize` (gather with
  `IndexOutOfRange` bounds check). Private to the crate, re-exposed
  via `Codebook::{quantize_into, dequantize_into}`.

New fixture tooling + LFS surface:
- `scripts/generate_rust_fixtures.py` gained `codebook` and `quantize`
  subcommands. The former writes the shared
  `training_n10000_d64.f32.bin` corpus plus one
  `expected_bw{2,4,8}_seed42.f32.bin` per bit width; the latter writes
  the shared `values_n10000.f32.bin` corpus (`default_rng(7)`) plus
  one `expected_bw{2,4,8}_seed42.u8.bin` of Python-computed indices.
- `rust/xtask/src/main.rs` learned `fixtures refresh-codebook`,
  `fixtures refresh-quantize`, and an extended `refresh-all` that now
  chains hashes → rotation → codebook → quantize.
- `.gitattributes` routes both new fixture dirs through Git LFS.

New tests — 16 additions on top of the Phase 13 baseline:
- 12 `codebook` tests: four construction invariants, the bw=2/4/8
  training parity sweep against the Python fixtures, two round-trip
  tests, the dequantize out-of-range rejection, `new_accepts_bw2_and_bw8_bounds`,
  and a ChaCha-seeded randomized scan (256 batches × up to 512 values)
  confirming indices stay in `[0, 16)`.
- 4 `quantize` tests: bw=2/4/8 byte parity against Python on a
  10 000-value corpus (30 000 indices total) plus a bw=4 dequantize
  round-trip that every output value belongs to the codebook entry
  set.

Workspace `cargo xtask fmt`, `cargo xtask lint`, `cargo xtask test`,
`cargo build -p tinyquant-core --no-default-features`, and
`cargo build -p tinyquant-core --target thumbv7em-none-eabihf
--no-default-features` are all green. `cargo xtask fixtures refresh-all`
leaves `git status` clean against the committed fixtures. Proptest was
*not* adopted because its transitive dependency tree now requires
`edition2024` (Rust 1.85+) while the workspace MSRV is 1.81; the
`rand_chacha`-driven deterministic scan is the drop-in replacement.

Deferred by design (not in Phase 14):
- Bit packing for on-disk index storage — Phase 16.
- Residual correction and the `Codec` service — Phase 15.
- SIMD quantize/dequantize kernels — Phase 20.

## [2026-04-10] maintain | Propagate Phase 14 findings across the wiki

Doc-only sweep that carries the Phase 14 execution observations into
the pages that are read *before* the implementation notes, so future
phase work picks them up without having to hunt.

- [[design/rust/README|Rust Port Design Overview]] — appended
  [[design/rust/phase-14-implementation-notes|Phase 14 Implementation
  Notes]] to the reading order.
- [[index|Wiki index]] — added rows for the Phase 14 plan and the
  Phase 14 implementation-notes page.
- [[design/rust/type-mapping|Type Mapping]] §Codebook — updated the
  sketched `Codebook::train` signature to match what landed
  (`(&[f32], &CodecConfig)`, no row/col params) and rewrote the
  "Numerical parity notes" bullets to point at the actual frozen
  fixtures across `bw ∈ {2, 4, 8}` rather than a hypothetical
  "1 000 random seeds" scheme.
- [[design/rust/numerical-semantics|Numerical Semantics]]
  §Quantization + §Codebook training — replaced the "256 random
  inputs at each supported bit width" claim with a pointer at the
  committed `expected_bw{2,4,8}_seed42.{f32,u8}.bin` fixtures, noted
  the `libm::floor` / `f32::total_cmp` details that make the path
  `no_std`-safe, and documented the `scalar_quantize` delegation
  contract.
- [[design/rust/testing-strategy|Testing Strategy]] — added a
  `[!warning]` callout documenting that `proptest` is currently
  blocked by MSRV 1.81 and showing the
  `rand_chacha::ChaCha20Rng::seed_from_u64(...)` substitute pattern
  (with the exact Phase 14 `codebook.rs` test as a template). The
  callout also names the re-entry condition (workspace MSRV
  crossing 1.85 or a proptest release compatible with 1.81) so the
  decision can be reviewed deliberately.
- [[design/rust/risks-and-mitigations|Risks and Mitigations]] §R12
  — corrected the MSRV check job from `cargo +1.78.0` to
  `cargo +1.81.0` and added a "Concrete incidents so far"
  subsection naming the Phase 14 proptest incident, its root cause
  (`getrandom 0.4.2` / `rustix` / `edition2024`), the chosen
  interim pattern, and the re-entry path.
- [[classes/codebook|Codebook]] — added a "Rust port" section that
  tabulates the Python-to-Rust field and method mapping, calls out
  the `Arc<[f32]>` storage + `PartialEq`-by-bits contract, and
  links to the Phase 14 implementation notes and type-mapping
  entry.
- [[qa/unit-tests/test-codebook|Codebook unit tests]] — added a
  "Rust integration tests" section listing every Phase 14 test with
  its assertion, explaining that they run alongside (not instead
  of) the existing Python unit tests and function as the
  byte-parity gate. Notes the LFS fixture layout and the xtask
  refresh command.

No code or fixtures changed in this commit; only wiki prose. The
non-`docs/` markdown surface continues to pass strict markdownlint,
and Obsidian-flavored constructs stay confined to the vault.

## [2026-04-10] retrospective | Phase 14 lessons learned — PR-cycle discoveries

Phase 14 shipped cleanly on the first code-review pass, but the
**PR CI cycle** surfaced three pre-existing bugs from Phase 13 that
had been silently rotting on `main`. All three were fixed as part
of PR #12 but belong to Phase 13 scope, so they are tracked
separately as follow-up debt.

Captured in docs/:

- [[design/rust/phase-14-implementation-notes|Phase 14
  Implementation Notes]] §Lessons learned — added seven lessons
  (L1–L7) covering pre-merge CI as the real quality gate, LFS
  hydration defaults, design-vs-implementation drift, cross-
  platform `faer` QR determinism, `f32` literal tie-break gotchas
  in round-trip tests, test-framework MSRV creep, and the
  recurring clippy-profile pitfalls. Each lesson ends with a
  "What to do differently" line so Phase 15+ can follow it
  without re-reading the full retrospective. Also added a
  `## CI follow-ups queued after Phase 14` section listing the
  three Phase 13 remediation items that must clear before Phase
  15 builds on top of them (determinism contract in
  `RotationMatrix::build`, toolchain drift in `rust-ci.yml`, and
  the phase-end CI health check).
- [[design/rust/ci-cd|CI/CD]] §Caching — added a
  `[!warning] Design-implementation drift caught in Phase 14`
  callout explaining that this doc's "lfs: true on every job"
  claim was not actually true in `rust-ci.yml` between Phase 11
  and Phase 14, why nobody noticed, and how it was fixed (commit
  `13e888d`).
- [[design/rust/risks-and-mitigations|Risks and Mitigations]] —
  added three new risk rows and their detailed mitigations:
  - **R19** — `faer` parallel-kernel cross-platform
    nondeterminism on Rust-canonical fixtures (root cause of the
    `seed_42_dim_768` failure), short-term
    `RAYON_NUM_THREADS: "1"` workaround, and the long-term
    `Parallelism::None` fix that a Phase 13 remediation PR should
    land.
  - **R20** — Design-doc drift from actual YAML / Rust source.
    Names the two concrete drifts Phase 14 caught (`ci-cd.md`
    §Caching and `testing-strategy.md` §Property tests) and the
    edit-time discipline needed to prevent recurrence.
  - **R21** — Trusted-but-unobserved CI workflows. Introduces a
    phase-exit checklist item
    (`gh run list --workflow rust-ci.yml --branch main --limit 5`
    must show zero failures) and a "no green-locally claims" rule
    for implementation-notes pages.
- [[design/rust/numerical-semantics|Numerical Semantics]] §Rotation
  matrix — added a `[!warning] Cross-platform faer QR
  nondeterminism (R19)` callout so anyone reading the rotation
  parity plan in the future cannot miss the determinism-contract
  requirement, plus pointers to the risks and implementation-notes
  entries.

Process lesson summary (one-liner each, for scanability):

1. Pre-merge CI is the quality gate, not local tests.
2. `actions/checkout@v4` defaults to `lfs: false` — must opt in.
3. Design docs are not CI config; verify testable claims against
   the YAML at edit time.
4. Cross-platform bit-exact parity needs a determinism contract
   in code, not a CI env-var workaround.
5. Dequantized round-trip tests must compare against
   `cb.entries()[k]`, not hand-typed `f32` literals.
6. Test-framework MSRV creep is real — `cargo tree` any new
   dev-dep before committing.
7. The crate's clippy profile has recurring catches worth
   memorising (`bool_to_int_using_if`, `explicit_iter_method`,
   `missing_fields_in_debug`, `redundant_pub_crate`,
   `trivially_copy_pass_by_ref`, `cast_*`).

No code or fixtures touched in this commit; only wiki prose. The
non-`docs/` markdown surface continues to pass strict markdownlint,
and Obsidian-flavored constructs stay confined to the vault.

## [2026-04-11] lint | orphan-page audit — added missing incoming links for 4 pages

Ran a full link-graph audit of the `docs/` vault. Found four pages
with no incoming wikilinks from any other document:

1. [[design/rust/phase-15-implementation-notes|Phase 15 Implementation
   Notes]] — existed but was not listed in `index.md` or
   `design/rust/README.md`, unlike the Phase 13 and 14 counterparts.
2. [[superpowers/plans/2026-04-08-phase-05-backend-layer|Phase 5
   Execution Plan]] — agentic task checklist with no path from the
   wiki.
3. [[superpowers/plans/2026-04-09-readme-gfm-refresh|README GFM
   Refresh Plan]] — agentic task checklist with no path from the wiki.
4. [[superpowers/plans/2026-04-10-phase-12-shared-types-and-errors|Phase
   12 Execution Plan]] — agentic task checklist with no path from the
   wiki.

Changes made (links added, no content changed):

- [[index|Wiki Index]] — added Phase 15 Implementation Notes row to
  the Roadmap and Plans table; added three superpowers execution-plan
  rows to the Specs table.
- [[design/rust/README|Rust Port Design Overview]] — added item 19
  (Phase 15 Implementation Notes) to the reading order.
- [[plans/phase-05-backend-layer|Phase 5: Backend Layer]] — added
  Phase 5 Execution Plan link to the See also section.
- [[plans/rust/phase-12-shared-types-and-errors|Phase 12: Shared Types
  and Error Enums]] — added Phase 12 Execution Plan link to the See
  also section.

All four pages are now reachable via at least one incoming wikilink.

## [2026-04-11] audit | Holistic link audit — broken links, orphans, and index completeness

Ran a full holistic audit of all wikilinks and markdown links across the
`docs/` vault. Four categories of issues found and resolved:

**Broken wikilinks:** Zero genuine broken wikilinks outside code fences.
All 11 candidates from the raw scanner were false positives (bash `[[ ]]`
conditionals and Cargo `[[bin]]` / `[[example]]` tokens inside fenced
code blocks or inline backtick spans).

**Orphaned pages:** Found 4 pages with no incoming links:

1. `research/tinyquant-research/better-router-integration.md` — the
   "Raw sources available" section in [[index]] used a backtick path
   instead of a wikilink. Converted to
   `~~[[research/tinyquant-research/better-router-integration]]~~`.

2. `sources/turbo-quant-deep-research-report.md`,
   `sources/turboquant-integration.md`, and
   `sources/vector-quantization-paper-synthesis.md` — the bare stems
   `[[turbo-quant-deep-research-report]]` etc. in [[index]], [[log]],
   and [[entities/TinyQuant|TinyQuant]] all resolved to the `research/`
   file with the same stem, leaving the `sources/` counterpart with zero
   incoming links. Fixed by switching all three sites to full-path links:
   `[[sources/turbo-quant-deep-research-report\|...]]` etc.

**Index completeness:** Found 25 wiki pages not listed in [[index]]:

- 16 individual `design/rust/` detail pages (goals-and-non-goals through
  risks-and-mitigations) — the Design table had only the README; added
  one row per page.
- 8 `plans/rust/` phase plans 15–22 — Roadmap and Plans table ended at
  Phase 14; added rows for Phases 15–22.

**Wikilink text drift:** Spot-checked all `[[target|display]]` links for
phase-number mismatches. The only discrepancies are zero-padding artefacts
(`phase-01` filename vs "Phase 1" display text), which are cosmetically
correct and not misleading.

All pages now have zero orphans and zero broken wikilinks (scanner + manual
review).

## [2026-04-13] operation | Phase 23 — Python reference demotion landed

Executed Phase 23 of the Rust port
([[plans/rust/phase-23-python-reference-demotion|plan]]) across three
commit slices. Step 1 rewrote test imports from `tinyquant_cpu` to
`tinyquant_py_reference` (red snapshot — no source moved yet). Step 2
`git mv`'d `src/tinyquant_cpu/` to
`tests/reference/tinyquant_py_reference/` with history preserved,
leaving no shipping source under `src/`. Step 3 updated
`pyproject.toml` — cleared `[tool.hatch.build.targets.wheel].packages`,
extended `pythonpath` to `tests/reference`, and rewrote coverage paths
— bringing the full 214-test suite back to green under the new name.
Step 4 added the `build-package-does-not-leak-reference` CI job to
`.github/workflows/ci.yml`; with hatchling `≥ 1.25` + `bypass-selection
= true` the `python -m build` invocation emits an empty metadata-only
wheel rather than failing, so the guard was structured as a
leakage-check on any produced wheel rather than a fail-on-success
assertion. Step 5 scaffolded `tests/parity/` (three files — empty
`__init__.py`, `conftest.py` with session-scoped `ref` / `rs`
fixtures, `test_cross_impl.py` with four classes and eight tests
under `pytestmark = pytest.mark.parity`). The `rs` fixture skips with
`Rust-backed tinyquant_cpu not installed` until Phase 24 installs the
fat wheel. Step 6 updated the prose surface: `AGENTS.md`, `CLAUDE.md`,
`README.md`, `.github/README.md`, [[entities/TinyQuant]], this log,
[[roadmap]], [[design/rust/testing-strategy]],
[[qa/unit-tests/README|qa/unit-tests]],
[[CI-plan/workflow-definition]], [[CD-plan/release-workflow]],
`scripts/ci_local_simulate.sh`, `scripts/generate_rust_fixtures.py`,
`CHANGELOG.md`, and added
[[entities/python-reference-implementation|python-reference-implementation]]
and [[design/rust/phase-23-implementation-notes]]. Phase 23 is a pure
refactor — no PyPI publish, no tag, version string stays at `0.1.1`.
