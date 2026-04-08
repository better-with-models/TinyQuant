---
title: Quality Gates
tags:
  - ci
  - quality
  - gates
date-created: 2026-04-08
status: active
category: ci-cd
---

# Quality Gates

> [!info] Purpose
> Defines what blocks a merge, what is advisory, and the escalation path
> for gate failures.

## Gate classification

### Hard gates (block merge)

| Gate | Tool | Threshold | Rationale |
|------|------|-----------|-----------|
| Lint errors | `ruff check` | 0 errors | [[design/architecture/linting-and-tooling\|Linting policy]] |
| Format violations | `ruff format --check` | 0 violations | Consistent formatting |
| Type errors | `mypy --strict` | 0 errors | [[design/architecture/type-safety\|Type Safety]] |
| Bare suppressions | `grep` check | 0 matches | Every suppression must be justified |
| Markdown lint (non-docs) | `markdownlint-cli2` | 0 errors | Portable markdown |
| Unit test failures | `pytest` | 0 failures | Correctness |
| Architecture violations | `pytest tests/architecture/` | 0 failures | [[design/architecture/high-coherence\|Coherence]] |
| Integration test failures | `pytest tests/integration/` | 0 failures | Boundary correctness |
| E2E test failures | `pytest tests/e2e/` | 0 failures | Pipeline correctness |
| Overall coverage | `pytest-cov` | >= 90% | Minimum test coverage |
| Codec coverage | `pytest-cov` | >= 95% | Core layer must be well-tested |
| Corpus coverage | `pytest-cov` | >= 90% | Aggregate layer must be well-tested |
| Package build | `python -m build` | Succeeds | Distributable artifact |
| Distribution check | `twine check` | 0 warnings | Valid PyPI metadata |

### Soft gates (advisory, PR review)

| Gate | Method | Action on failure |
|------|--------|------------------|
| Complexity warnings (CC 5-7) | `ruff` monitoring | Reviewer discretion |
| File length > 200 lines | Manual review | Reviewer discretion |
| Missing docstring on private helper | `ruff` rule D (configurable) | Reviewer discretion |
| Backend coverage < 80% | `pytest-cov` | Comment on PR |

### Release gates (pre-release only)

| Gate | Tool | Threshold |
|------|------|-----------|
| Calibration: Pearson rho | `pytest tests/calibration/` | >= 0.995 (4-bit + residuals) |
| Calibration: compression ratio | `pytest tests/calibration/` | >= 7x (4-bit) |
| Calibration: determinism | `pytest tests/calibration/` | Byte-identical across runs |
| Sphinx doc build | `sphinx-build` | 0 warnings |
| Version tag matches pyproject.toml | Release workflow check | Exact match |

## Failure handling

### When a hard gate fails

1. The PR cannot merge (GitHub branch protection enforces this)
2. The CI log shows which gate failed and why
3. The developer fixes the issue and pushes
4. CI reruns automatically

### When a soft gate fails

1. The CI job passes but leaves a comment or annotation
2. The reviewer decides whether to require a fix before merge
3. Soft gates are tracked as trends — if a soft gate fails frequently, consider
   promoting it to hard

### When a release gate fails

1. The release workflow stops before publishing
2. The failure is investigated: is it a real regression or a data issue?
3. If real: fix and re-run
4. If data issue: update calibration dataset and re-run

## Escalation

| Situation | Action |
|-----------|--------|
| Hard gate consistently blocks valid PRs | Review the threshold; tighten or loosen with a documented reason |
| Soft gate is ignored by reviewers | Promote to hard gate or remove it |
| Flaky test fails intermittently | Quarantine the test; fix root cause; restore within 1 week |
| Coverage drops below threshold | Do not lower the threshold; add missing tests |

> [!warning] Never bypass
> Hard gates are never bypassed. If a gate is wrong, update the gate
> definition in the CI workflow — do not use `[skip ci]` or merge without
> checks.

## See also

- [[CI-plan/README|CI Plan]]
- [[CI-plan/pipeline-stages|Pipeline Stages]]
- [[CI-plan/workflow-definition|Workflow Definition]]
- [[qa/verification-plan/README|Verification Plan]]
- [[design/architecture/linting-and-tooling|Linting and Tooling]]
