---
title: Workflow Definition
tags:
  - ci
  - github-actions
  - workflow
date-created: 2026-04-08
status: active
category: ci-cd
---

# Workflow Definition

> [!info] Purpose
> Specification for `.github/workflows/ci.yml` — the primary CI workflow for
> TinyQuant. This document defines the exact workflow structure; the actual
> YAML file should match this specification.

## Workflow file

```text
.github/workflows/ci.yml
```

## Triggers

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

- **Push to main:** validates trunk after merge
- **Pull request to main:** validates before merge (required status check)
- **Concurrency:** cancels in-progress runs for the same branch to save compute

## Environment variables

```yaml
env:
  PYTHON_VERSION: "3.12"
  FORCE_COLOR: "1"
```

## Job graph

```yaml
jobs:
  lint:           # Stage 1a — parallel
  markdown-lint:  # Stage 1b — parallel
  typecheck:      # Stage 2 — needs: [lint]
  test:           # Stage 3+4 — needs: [typecheck, markdown-lint]
  build-artifact: # Stage 6 — needs: [test]
```

## Job specifications

### `lint`

```yaml
  lint:
    name: Lint & Format
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: ${{ env.PYTHON_VERSION }}

      - name: Install ruff
        run: pip install ruff

      - name: Ruff check
        run: ruff check .

      - name: Ruff format check
        run: ruff format --check .

      - name: Check bare suppressions
        run: |
          # Fail if any # noqa or # type: ignore lacks an explanation
          ! grep -rn '# noqa$' src/ tests/ || exit 1
          ! grep -rn '# type: ignore$' src/ tests/ || exit 1
```

### `markdown-lint`

```yaml
  markdown-lint:
    name: Markdown Lint
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@v4

      - name: Run markdownlint
        uses: DavidAnson/markdownlint-cli2-action@v19
        with:
          globs: |
            **/*.md
            !docs/**
```

### `typecheck`

```yaml
  typecheck:
    name: Type Check
    runs-on: ubuntu-latest
    needs: [lint]
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: ${{ env.PYTHON_VERSION }}

      - name: Install dependencies
        run: pip install -e ".[dev]"

      - uses: actions/cache@v4
        with:
          path: .mypy_cache
          key: mypy-${{ hashFiles('pyproject.toml') }}-${{ github.sha }}
          restore-keys: mypy-${{ hashFiles('pyproject.toml') }}

      - name: mypy strict
        run: mypy --strict .
```

### `test`

```yaml
  test:
    name: Test (Python ${{ matrix.python-version }})
    runs-on: ubuntu-latest
    needs: [typecheck, markdown-lint]
    timeout-minutes: 15
    strategy:
      fail-fast: true
      matrix:
        python-version: ["3.12", "3.13"]
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.python-version }}

      - uses: actions/cache@v4
        with:
          path: ~/.cache/pip
          key: pip-${{ matrix.python-version }}-${{ hashFiles('pyproject.toml') }}
          restore-keys: pip-${{ matrix.python-version }}-

      - name: Install dependencies
        run: pip install -e ".[dev]"

      - name: Unit tests
        run: pytest tests/codec/ tests/corpus/ tests/backend/ -x --tb=short

      - name: Architecture tests
        run: pytest tests/architecture/ -x --tb=short

      - name: Integration tests
        run: pytest tests/integration/ -x --tb=short

      - name: E2E tests
        run: pytest tests/e2e/ -x --tb=short

      - name: Coverage report
        run: |
          pytest --cov=tinyquant_cpu --cov-report=xml --cov-fail-under=90
          pytest --cov=tinyquant_cpu/codec --cov-report=term --cov-fail-under=95
          pytest --cov=tinyquant_cpu/corpus --cov-report=term --cov-fail-under=90

      - name: Upload coverage
        if: matrix.python-version == '3.12'
        uses: actions/upload-artifact@v5
        with:
          name: coverage-report
          path: coverage.xml
          retention-days: 14
```

### `build-artifact`

```yaml
  build-artifact:
    name: Build Package
    runs-on: ubuntu-latest
    needs: [test]
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: ${{ env.PYTHON_VERSION }}

      - name: Install build tools
        run: pip install build

      - name: Build wheel and sdist
        run: python -m build

      - name: Verify distributions
        run: |
          pip install twine
          twine check dist/*

      - name: Upload artifact
        uses: actions/upload-artifact@v5
        with:
          name: dist
          path: dist/
          retention-days: 30
```

## Required status checks

Configure in GitHub branch protection for `main`:

| Check | Required |
|-------|----------|
| `Lint & Format` | Yes |
| `Markdown Lint` | Yes |
| `Type Check` | Yes |
| `Test (Python 3.12)` | Yes |
| `Test (Python 3.13)` | Yes |
| `Build Package` | Yes |

All checks must pass before a PR can merge.

## See also

- [[CI-plan/README|CI Plan]]
- [[CI-plan/pipeline-stages|Pipeline Stages]]
- [[CI-plan/quality-gates|Quality Gates]]
- [[design/architecture/linting-and-tooling|Linting and Tooling]]
