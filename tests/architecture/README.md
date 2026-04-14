# tests/architecture

This directory contains architecture enforcement tests (V-01). There is one test
file, `test_dependency_direction.py`, which uses `importlib` and `sys.modules`
inspection to verify that the three bounded contexts of the reference
implementation — `codec`, `corpus`, and `backend` — obey their prescribed
dependency order. The suite also checks that each subpackage `__init__`
exports exactly the names listed in `__all__` and nothing more. These tests are
kept separate from unit tests because they probe import-time behaviour rather
than runtime logic, and a failure here indicates an architectural violation, not
a functional regression.

## What lives here

- `test_dependency_direction.py` — three test classes:
  - `TestDependencyDirection` — asserts that importing `tinyquant_py_reference.codec`
    does not pull in any `corpus` or `backend` modules, and that importing `corpus`
    does not pull in any `backend` modules.
  - `TestImportGraph` — imports all three packages together, builds a cross-package
    dependency map from `sys.modules`, then asserts no reverse edges exist in the
    DAG `codec -> corpus -> backend`.
  - `TestExports` — parametrized over the three subpackages; verifies that every
    name in `__all__` is accessible and every non-private attribute is listed in
    `__all__`.
- `__init__.py` — empty package marker.

## How this area fits the system

These tests import `tinyquant_py_reference.*` directly from `sys.path` (which
pytest populates via `pythonpath = ["tests/reference"]` in `pyproject.toml`).
They have no runtime dependencies on fixtures from `tests/conftest.py`. A
`_clean_tinyquant_imports()` context manager wraps each test that examines
import side-effects so that module state is restored between tests. The tests
fail fast (the suite runs with `-x`) if any cross-layer import leak is
introduced.

## Common edit paths

- Extend `TestExports` or `TestDependencyDirection` when a new bounded context is
  added to `tinyquant_py_reference`.
- Update the allowed DAG in `TestImportGraph` if the architecture intentionally
  introduces a new dependency direction.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
