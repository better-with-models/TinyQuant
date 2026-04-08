---
title: CI Plan
tags:
  - ci
  - pipeline
  - github-actions
  - meta
date-created: 2026-04-08
status: active
category: ci-cd
---

# CI Plan

> [!info] Purpose
> Continuous integration strategy for TinyQuant using GitHub Actions. Defines
> the pipeline stages, quality gates, and workflow configuration that enforce
> the architecture policies in [[design/architecture/README|Architecture]].

## Reading order

1. [[CI-plan/pipeline-stages|Pipeline Stages]] — fail-fast stage ordering and job dependency graph
2. [[CI-plan/workflow-definition|Workflow Definition]] — the actual `.github/workflows/ci.yml` specification
3. [[CI-plan/quality-gates|Quality Gates]] — what blocks merge and what is advisory

## Principles

| Principle | Application to TinyQuant |
|-----------|------------------------|
| **Fail fast** | Lint and format run before tests; type checking before integration |
| **Pipeline as code** | All CI config lives in `.github/workflows/`, reviewed like code |
| **Short-lived branches** | GitHub Flow; PRs merge to `main` within days |
| **Automate everything** | No manual steps in the CI pipeline; manual gates only in CD |
| **Warnings as errors** | Zero-tolerance: any warning fails the pipeline |

## See also

- [[CI-plan/pipeline-stages|Pipeline Stages]]
- [[CI-plan/workflow-definition|Workflow Definition]]
- [[CI-plan/quality-gates|Quality Gates]]
- [[CD-plan/README|CD Plan]]
- [[qa/verification-plan/README|Verification Plan]]
