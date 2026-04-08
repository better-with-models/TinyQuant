---
title: "Phase 1: Project Scaffolding"
tags:
  - plans
  - phase-1
  - scaffolding
date-created: 2026-04-08
status: complete
date-completed: 2026-04-08
category: planning
---

# Phase 1: Project Scaffolding

> [!info] Goal
> Create the Python project structure, tooling configuration, and package
> skeleton so that `pip install -e ".[dev]"`, `ruff check .`, `mypy .`, and
> `pytest` all work on an empty codebase.

## Prerequisites

- None (first phase)

## Deliverables

### Files to create

| File | Purpose |
|------|---------|
| `pyproject.toml` | Package metadata, dependencies, tool config (ruff, mypy, pytest) |
| `src/tinyquant/__init__.py` | Top-level package with `__version__` |
| `src/tinyquant/py.typed` | PEP 561 marker for mypy consumers |
| `src/tinyquant/codec/__init__.py` | Empty codec package |
| `src/tinyquant/corpus/__init__.py` | Empty corpus package |
| `src/tinyquant/backend/__init__.py` | Empty backend package |
| `src/tinyquant/backend/adapters/__init__.py` | Empty adapters sub-package |
| `src/tinyquant/_types.py` | Shared type aliases (empty stubs) |
| `tests/__init__.py` | Test root |
| `tests/conftest.py` | Shared fixtures (empty) |
| `tests/codec/__init__.py` | Codec test package |
| `tests/corpus/__init__.py` | Corpus test package |
| `tests/backend/__init__.py` | Backend test package |
| `tests/integration/__init__.py` | Integration test package |
| `tests/e2e/__init__.py` | E2E test package |
| `tests/architecture/__init__.py` | Architecture test package |
| `tests/calibration/__init__.py` | Calibration test package |
| `.pre-commit-config.yaml` | Pre-commit hooks for ruff + mypy |
| `CHANGELOG.md` | Initial changelog skeleton |

### pyproject.toml specification

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "tinyquant"
version = "0.1.0"
description = "CPU-only vector quantization codec for embedding storage compression"
requires-python = ">=3.12"
license = "Apache-2.0"
dependencies = [
    "numpy>=1.26",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0",
    "pytest-cov>=5.0",
    "hypothesis>=6.100",
    "mypy>=1.10",
    "ruff>=0.5",
    "twine>=5.0",
    "build>=1.0",
]
pgvector = [
    "psycopg[binary]>=3.1",
]

[tool.ruff]
target-version = "py312"
line-length = 88

[tool.ruff.lint]
select = ["E", "W", "F", "I", "N", "UP", "B", "C901", "S", "SIM",
          "TCH", "ANN", "D", "RUF"]

[tool.ruff.lint.mccabe]
max-complexity = 7

[tool.ruff.lint.pydocstyle]
convention = "google"

[tool.mypy]
python_version = "3.12"
strict = true
warn_return_any = true
warn_unreachable = true
no_implicit_reexport = true

[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--import-mode=importlib -x --tb=short"
```

## Steps (TDD order)

1. Create directory structure: `src/tinyquant/`, `tests/`, all `__init__.py` files
2. Write `pyproject.toml` with full tool configuration
3. Write `src/tinyquant/__init__.py` with `__version__ = "0.1.0"`
4. Write `src/tinyquant/py.typed` (empty marker file)
5. Write `src/tinyquant/_types.py` with stub type aliases
6. Write `tests/conftest.py` (empty, with a docstring)
7. Write one smoke test: `tests/test_smoke.py` — `test_import_tinyquant`
8. Run `pip install -e ".[dev]"` — must succeed
9. Run `pytest` — smoke test must pass
10. Run `ruff check .` — must pass with zero errors
11. Run `mypy .` — must pass in strict mode
12. Write `.pre-commit-config.yaml`
13. Write `CHANGELOG.md` skeleton
14. Commit

## Verification

```bash
pip install -e ".[dev]"
ruff check . && ruff format --check .
mypy --strict .
pytest
```

All four commands must succeed with zero errors.

## Estimated scope

- ~19 files created
- ~1 test
- ~150 lines of configuration

## See also

- [[roadmap|Implementation Roadmap]]
- [[plans/phase-02-codec-value-objects|Phase 2: Codec Value Objects]]
- [[design/architecture/namespace-and-module-structure|Namespace and Module Structure]]
- [[design/architecture/linting-and-tooling|Linting and Tooling]]
