# Changelog

All notable changes to TinyQuant will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Pure-Python implementation demoted to a test-only reference at
  `tests/reference/tinyquant_py_reference/`. It is no longer shipped
  on PyPI. The last pure-Python release remains `tinyquant-cpu==0.1.1`.
  Phase 24 will reclaim the `tinyquant-cpu` name with a Rust-backed
  fat wheel at version `0.2.0`.

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
