---
title: Versioning and Changelog
tags:
  - cd
  - versioning
  - changelog
  - semver
date-created: 2026-04-08
status: active
category: ci-cd
---

# Versioning and Changelog

> [!info] Purpose
> Semantic versioning rules and changelog generation strategy for TinyQuant
> releases.

## Semantic versioning

TinyQuant follows [Semantic Versioning 2.0.0](https://semver.org/):

```text
MAJOR.MINOR.PATCH

MAJOR — breaking changes to the public API
MINOR — new features, backward-compatible
PATCH — bug fixes, backward-compatible
```

### What constitutes a breaking change

| Change | Breaking? | Version bump |
|--------|-----------|-------------|
| Remove a public class or method | Yes | MAJOR |
| Change a method signature (required params) | Yes | MAJOR |
| Change CompressedVector binary format | Yes | MAJOR |
| Change CodecConfig field semantics | Yes | MAJOR |
| Add a new public method | No | MINOR |
| Add a new compression policy variant | No | MINOR |
| Add a new backend adapter | No | MINOR |
| Add an optional parameter with default | No | MINOR |
| Fix a bug in quantization math | No | PATCH |
| Improve compression fidelity without API change | No | PATCH |
| Update dependencies | No | PATCH |

### Pre-release versions

```text
v0.1.0       — initial development (API unstable)
v0.1.0-alpha.1  — alpha pre-release
v0.1.0-beta.1   — beta pre-release
v0.1.0-rc.1     — release candidate
v1.0.0       — first stable release (API contract begins)
```

> [!tip] Before 1.0
> During `0.x` development, MINOR bumps may include breaking changes.
> The stable API contract begins at `1.0.0`.

## Version source of truth

```toml
# pyproject.toml
[project]
name = "tinyquant"
version = "0.1.0"
```

The version in `pyproject.toml` is the single source of truth. The release
workflow verifies that the git tag matches this version.

## Release process

1. **Prepare:** update `pyproject.toml` version and `CHANGELOG.md`
2. **Commit:** `git commit -m "chore: bump version to 0.2.0"`
3. **Tag:** `git tag v0.2.0`
4. **Push:** `git push origin main --tags`
5. **Automated:** release workflow triggers on tag push

## Changelog format

```markdown
# Changelog

## [0.2.0] - 2026-05-15

### Added
- `CompressionPolicy.FP16` variant for half-precision storage
- `BruteForceBackend.clear()` convenience method

### Changed
- `Codebook.train()` now accepts an optional `method` parameter

### Fixed
- `CompressedVector.from_bytes()` now rejects truncated input correctly

## [0.1.0] - 2026-04-15

### Added
- Initial codec layer: CodecConfig, RotationMatrix, Codebook, CompressedVector, Codec
- Initial corpus layer: Corpus, VectorEntry, CompressionPolicy, domain events
- BruteForceBackend reference implementation
- SearchBackend protocol
```

### Changelog rules

- Entries are grouped by `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`
- Each entry references the user-facing behavior, not internal implementation
- Breaking changes are highlighted with **BREAKING:** prefix
- The changelog is maintained manually, not auto-generated from commits
  (commit messages are for developers; changelog entries are for consumers)

## Automated release notes

The GitHub Release (created by the release workflow) includes auto-generated
notes from commit messages between tags. These supplement but do not replace
the curated `CHANGELOG.md`.

```yaml
# In the release workflow
- uses: softprops/action-gh-release@v2
  with:
    body: |
      See [CHANGELOG.md](CHANGELOG.md) for details.

      ## Commits since last release
      ${{ steps.notes.outputs.notes }}
```

## Release cadence

| Phase | Cadence |
|-------|---------|
| Pre-1.0 development | Release when meaningful features or fixes accumulate |
| Post-1.0 stable | Patch releases as needed; minor releases monthly if features accumulate |

No scheduled releases. Ship when ready, not on a calendar.

## See also

- [[CD-plan/README|CD Plan]]
- [[CD-plan/release-workflow|Release Workflow]]
- [[CD-plan/artifact-management|Artifact Management]]
