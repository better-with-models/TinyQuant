# Changelog

All notable changes to TinyQuant will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `markdownlint-obsidian` pre-commit hook (`.pre-commit-config.yaml`)
  scoped to `docs/**/*.md` except `docs/research/`, gated at
  `alisonaquinas/markdownlint-obsidian@markdownlint-obsidian-cliv1.0.6`
- `.github/workflows/docs-lint.yml` CI job running
  `markdownlint-obsidian-cli` against the docs vault on every push and
  pull request that touches `docs/` or the lint configuration
- `.obsidian-linter.jsonc` root config shared by the pre-commit hook and
  the CI job, excluding `docs/research/`, `docs/.obsidian/`, and
  `.worktrees/`
- Module-level `//!` docstrings on the four Rust integration-test files
  that were missing them (`tinyquant-core/tests/codec_fixture_parity.rs`,
  `codec_service.rs`, `compressed_vector.rs`, `residual.rs`)
- Module docstring on `scripts/verify_pre_commit.py`
- Full-maturity `README.md`, `AGENTS.md`, and `CLAUDE.md` stubs across
  every code and test subtree so the `/well-documented` audit now maps
  the actual repository layout rather than reporting empty subtrees
- Hand-refined `AGENTS.md` for each top-level code subtree (`rust/`,
  `src/`, `tests/`, `scripts/`) with real responsibilities, layout, and
  invariants
- PyO3 wheel + C ABI + CLI surfaces (Phase 22.A–C) under
  `rust/crates/tinyquant-{py,sys,cli}/`, with maturin abi3-py312
  packaging, cbindgen-generated `tinyquant.h`, and a `tinyquant`
  CLI exposing `codec train/compress/decompress`, `corpus
  ingest/search`, `info`, and `verify`
- `.github/workflows/rust-release.yml` matrix release workflow plus
  `rust/Dockerfile` distroless container image and
  `COMPATIBILITY.md` ledger for cross-package version pairing
  (Phase 22.D)
- `tests/reference/tinyquant_py_reference/` cross-implementation
  parity scaffold (`tests/parity/conftest.py`) with `rs` and `py`
  fixtures, ready for the Phase 24 fat wheel to plug in
- `src/tinyquant_cpu/` developer shim plus
  `scripts/packaging/templates/` and
  `scripts/packaging/assemble_fat_wheel.py` that fold the five
  per-arch wheels from Phase 22.A into a single
  `tinyquant_cpu-0.2.0-py3-none-any.whl` (Phase 24.1–24.2)
- `.github/workflows/python-fatwheel.yml` dry-run-by-default release
  workflow with a `release-gate` job and a publish-job byte-identical
  guard enforced by `cargo xtask check-sync-python` (Phase 24.3)
- Phase 24.4 design note at
  `docs/design/rust/phase-24-implementation-notes.md` recording the
  parity-audit wiring, AC trace, and 6 declared deviations from
  `phase-24-python-fat-wheel-official.md`.

### Changed

- Pure-Python implementation demoted to a test-only reference at
  `tests/reference/tinyquant_py_reference/`. It is no longer shipped
  on PyPI. The last pure-Python release remains `tinyquant-cpu==0.1.1`.
  Phase 24 reclaims the `tinyquant-cpu` name with a Rust-backed fat
  wheel at version `0.2.0`.
- Root `AGENTS.md` now documents the two markdown-lint surfaces (strict
  markdownlint for non-`docs/` and `markdownlint-obsidian` for the
  vault), an explicit docstring requirement per language, and the
  configuration files that back each check
- `docs/README.md` now describes the automated Obsidian lint layer next
  to the existing editorial lint guidance

## [0.1.1] - 2026-04-09

Documentation and CI maintenance release. No user-facing Python API
changes — the codec, corpus, and backend modules are byte-identical to
0.1.0.

### Added

- Rich GitHub-flavored landing page at `.github/README.md` with
  `<picture>` hero that switches between light and dark logo variants
  via `prefers-color-scheme`, a `> [!NOTE]` TL;DR callout, collapsible
  `<details>` table of contents, a "How it works" explainer, recipes
  as collapsible blocks with a rate-distortion comparison matrix, and
  alert admonitions for tuning warnings and prerequisites. GitHub
  auto-prefers this file over the root `README.md` for the repo
  landing page
- Canonical short tagline (*"CPU-only vector quantization codec for
  embedding storage compression."*) and elevator-pitch paragraph
  propagated across `README.md`, `.github/README.md`, `AGENTS.md`, and
  `CLAUDE.md`
- `## About TinyQuant` section in `AGENTS.md` with the canonical
  elevator-pitch paragraph
- `## Cross-file prose alignment` rule in `AGENTS.md` enforcing that
  tagline / elevator-pitch / headline benchmark numbers stay in sync
  across all four canonical identity files
- Live PyPI version badge in the root `README.md` replacing the stale
  static `version-0.1.0-green` badge
- Plan archives under `docs/superpowers/plans/` documenting the
  README refresh implementation

### Changed

- Tightened the root `README.md` lead from two paragraphs into the
  single canonical elevator-pitch paragraph, moving the headline
  benchmark numbers (8× at 4-bit, ρ ≈ 0.998, 95% top-5 recall) into
  the opening instead of the later "Why TinyQuant" section
- Propagated the `tinyquant_cpu` import name and `tinyquant-cpu`
  distribution name across the Obsidian docs vault (49 files updated
  in 158 mechanical substitutions, with wiki-links, frontmatter tags,
  and brand-named directories intentionally preserved)
- Refreshed `docs/CD-plan/release-workflow.md` to mirror the actual
  `release.yml` after the build-once-upload-many restructure, adding
  new sections documenting the `$GITHUB_OUTPUT` sidestep rationale,
  the `skip-existing` asymmetry between TestPyPI and PyPI, and the
  re-tag recovery runbook
- Upgraded GitHub Actions to Node 24: `actions/upload-artifact@v5`,
  `actions/download-artifact@v5`, replaced `softprops/action-gh-release`
  with the `gh` CLI (hardened against shell injection in commit
  subjects via `printf` + `nullglob` + `EXIT` trap)
- Narrowed the markdown-lint scope in both `scripts/verify_pre_commit.py`
  and `.github/workflows/ci.yml` to exclude `.github/` and `.venv/`
  alongside the existing exclusion of `docs/`. `.github/README.md`
  can now use inline HTML and alert admonitions freely; in-worktree
  virtualenvs no longer pull vendored upstream `LICENSE.md` files into
  the lint

### Fixed

- `verify-tag` step in `release.yml` now uses `sed` instead of a
  `python -c` multi-line command substitution that was silently
  producing empty `$GITHUB_OUTPUT` entries on the ubuntu-latest runner,
  and computes the version directly in each consuming step from
  `${GITHUB_REF_NAME#v}` to sidestep the issue entirely. Introduced
  during the 0.1.0 release cycle; fully resolved before this release

## [0.1.0] - 2026-04-08

### Added

- Codec module with compress/decompress pipeline: rotation
  preconditioning, scalar quantization, and optional residual correction
- CodecConfig immutable configuration with deterministic config hashing
- Codebook training via uniform quantile estimation
- CompressedVector with versioned binary serialization (bit-packed
  indices, LSB-first)
- RotationMatrix via QR decomposition of seeded random matrix
- Batch compress/decompress operations
- Corpus aggregate root with insert, decompress, and domain event
  emission
- Three compression policies: COMPRESS, PASSTHROUGH, FP16
- Domain events: CorpusCreated, VectorsInserted, CorpusDecompressed,
  CompressionPolicyViolationDetected
- BruteForceBackend with exhaustive cosine similarity search
- SearchBackend protocol for pluggable backend implementations
- PgvectorAdapter for PostgreSQL + pgvector wire format integration
- Custom domain errors: DimensionMismatchError, ConfigMismatchError,
  CodebookIncompatibleError, DuplicateVectorError
- Calibration test suite: score fidelity (VAL-01), compression ratio
  (VAL-02), determinism (VAL-03), research alignment (VAL-04)
- End-to-end test suite with 10 pipeline scenarios
- Architecture tests for dependency direction enforcement
- Property-based tests with Hypothesis
- Obsidian documentation wiki with design docs, behavior specs,
  validation plan, and research synthesis
