---
title: "Phase 8: CI/CD Workflows"
tags:
  - plans
  - phase-8
  - ci
  - cd
  - github-actions
date-created: 2026-04-08
status: pending
category: planning
---

# Phase 8: CI/CD Workflows

> [!info] Goal
> Implement the GitHub Actions workflow files specified in the CI and CD
> plans. After this phase, every push and PR runs the full CI pipeline,
> and tagged releases flow through to TestPyPI.

## Prerequisites

- [[plans/phase-07-architecture-e2e-tests|Phase 7]] complete (all tests pass locally)

## Deliverables

| File | What | Spec |
|------|------|------|
| `.github/workflows/ci.yml` | CI pipeline | [[CI-plan/workflow-definition\|Workflow Definition]] |
| `.github/workflows/release.yml` | Release pipeline | [[CD-plan/release-workflow\|Release Workflow]] |
| `.github/release-drafter.yml` | Release notes config | [[CD-plan/versioning-and-changelog\|Versioning]] |
| `.pre-commit-config.yaml` | Update with final hooks | [[design/architecture/linting-and-tooling\|Linting]] |

## Steps

### 1. CI workflow

1. Create `.github/workflows/ci.yml` matching the spec in [[CI-plan/workflow-definition|Workflow Definition]]
2. Jobs: `lint`, `markdown-lint`, `typecheck`, `test` (matrix), `build-artifact`
3. Concurrency group: cancel in-progress for same branch
4. Caching: pip and mypy caches
5. Coverage upload as artifact

### 2. Release workflow

1. Create `.github/workflows/release.yml` matching the spec in [[CD-plan/release-workflow|Release Workflow]]
2. Jobs: `verify-tag`, `calibration-tests`, `publish-testpypi`, `publish-pypi`, `github-release`
3. OIDC trusted publishing: `id-token: write` permission
4. Manual approval gate on `pypi` environment

### 3. Release drafter config

1. Create `.github/release-drafter.yml` with auto-labeling rules
2. Categories: Features, Bug Fixes, Maintenance

### 4. Pre-commit update

1. Update `.pre-commit-config.yaml` with ruff-pre-commit hooks
2. Add local mypy hook

### 5. Verify workflows

1. Validate YAML syntax
2. Push to a test branch and verify CI triggers
3. Verify job dependency graph matches [[CI-plan/pipeline-stages|Pipeline Stages]]

## Verification

- CI workflow triggers on push and PR
- All CI jobs pass (lint, typecheck, test, build)
- Release workflow is syntactically valid (verified by pushing a test tag to a fork, or by dry-run)
- `.pre-commit-config.yaml` runs locally with `pre-commit run --all-files`

## Estimated scope

- ~4 files created/modified
- ~300 lines of YAML
- No Python code changes

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-07-architecture-e2e-tests|Phase 7]]
- [[plans/phase-09-pgvector-adapter|Phase 9: Pgvector Adapter]]
- [[CI-plan/README|CI Plan]]
- [[CD-plan/README|CD Plan]]
