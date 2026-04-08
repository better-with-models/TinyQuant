---
title: Linting and Tooling
tags:
  - design
  - architecture
  - linting
  - ci
  - tooling
date-created: 2026-04-08
status: active
category: design
---

# Linting and Tooling

> [!info] Policy
> Strict linting on all code. Warnings are errors. Every check runs in CI
> and locally with the same configuration.

## Toolchain

| Tool | Purpose | Enforcement |
|------|---------|-------------|
| **ruff** | Linting and formatting (replaces flake8, isort, black) | CI gate + pre-commit hook |
| **mypy** | Static type checking in strict mode | CI gate (see [[architecture/type-safety\|Type Safety]]) |
| **pytest** | Test runner | CI gate |
| **pytest-cov** | Coverage reporting | CI gate with minimum threshold |
| **markdownlint-cli2** | Markdown linting outside `docs/` | CI gate (existing) |

## Ruff configuration

```toml
# pyproject.toml (excerpt)

[tool.ruff]
target-version = "py312"
line-length = 88

[tool.ruff.lint]
select = [
    "E",     # pycodestyle errors
    "W",     # pycodestyle warnings
    "F",     # pyflakes
    "I",     # isort
    "N",     # pep8-naming
    "UP",    # pyupgrade
    "B",     # flake8-bugbear
    "C901",  # mccabe complexity
    "S",     # flake8-bandit (security)
    "SIM",   # flake8-simplify
    "TCH",   # flake8-type-checking
    "ANN",   # flake8-annotations
    "D",     # pydocstyle (PEP 257)
    "RUF",   # ruff-specific rules
]

[tool.ruff.lint.mccabe]
max-complexity = 7

[tool.ruff.lint.pydocstyle]
convention = "google"
```

## Warnings as errors

> [!warning] No suppressions without justification
> Every `# noqa`, `# type: ignore`, or `# pragma: no cover` must include a
> comment explaining why the suppression is necessary. Bare suppressions are
> not permitted.

| Anti-pattern | Required instead |
|-------------|-----------------|
| `# noqa` | `# noqa: E501 — URL exceeds line length, cannot split` |
| `# type: ignore` | `# type: ignore[arg-type] — numpy overload not in stubs` |
| `# pragma: no cover` | `# pragma: no cover — defensive branch for corrupted data` |

**CI behavior:** all linting tools run with zero-warning tolerance. A single
unaddressed warning fails the pipeline.

## Pre-commit integration

```yaml
# .pre-commit-config.yaml (target)
repos:
  - repo: https://github.com/astral-sh/ruff-pre-commit
    hooks:
      - id: ruff
        args: [--fix]
      - id: ruff-format
  - repo: local
    hooks:
      - id: mypy
        name: mypy
        entry: mypy
        language: system
        types: [python]
        pass_filenames: false
```

## Local verification command

```bash
# Run the full check suite locally before committing
ruff check . && ruff format --check . && mypy . && pytest
```

This must produce the same result as CI. If it passes locally and fails in
CI (or vice versa), that is a tooling bug to fix immediately.

## Coverage policy

| Layer | Minimum coverage |
|-------|-----------------|
| `tinyquant/codec/` | 95% |
| `tinyquant/corpus/` | 90% |
| `tinyquant/backend/` | 80% |
| Overall | 90% |

Coverage is a floor, not a target. 100% coverage with weak assertions is
worse than 90% coverage with strong invariant checks.

## See also

- [[architecture/type-safety|Type Safety]]
- [[architecture/docstring-policy|Docstring Policy]]
- [[architecture/file-and-complexity-policy|File and Complexity Policy]]
- [[architecture/test-driven-development|Test-Driven Development]]
