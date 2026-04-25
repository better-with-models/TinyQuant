---
title: "Python Reference Implementation"
tags: [entities, python, reference, testing]
date-created: 2026-04-13
status: active
category: testing
---

# Python Reference Implementation

> [!info] Role
> The pure-Python + NumPy reference implementation of the TinyQuant
> codec. It is a **test-only** artifact: a differential oracle that
> anchors the Rust core and the Phase 24 fat wheel to the behavior
> that shipped as `tinyquant-cpu==0.1.1` on PyPI. It is never
> installed by end users.

## What it is

`tinyquant_py_reference` is the pure-Python implementation of
TinyQuant's codec, corpus, and backend layers. It was the original
shipping engine (PyPI name `tinyquant-cpu`, versions `0.1.0` and
`0.1.1`) and the implementation against which every Python test in
the repository was written. With Phase 23 it no longer ships.

Its public surface is frozen at the `v0.1.1` behavior. Any byte- or
numerics-level change to this package is a breaking change to the
contract the Rust core must honor, and is treated as such.

## Where it lives

`tests/reference/tinyquant_py_reference/` — a regular importable
Python package under the test tree. `pyproject.toml`'s
`[tool.pytest.ini_options].pythonpath` extension makes it reachable
from every pytest invocation; it is deliberately **not** on the
normal Python sys.path and is not picked up by
`python -m build`.

Legacy location (pre-Phase 23): `src/tinyquant_cpu/`. The Phase 23
refactor removed that tree in favor of the test-tree placement so
the package cannot be accidentally re-shipped.

## Why it exists

- **Behavior oracle for the Rust port.** Every Rust implementation
  decision that must be byte-equivalent to `v0.1.1` — config hashing,
  codebook training, quantization, serialization, corpus
  aggregate, brute-force backend — is validated by comparing Rust
  output against this reference. The Phase 23 plan spells out the
  rename at
  [[plans/rust/phase-23-python-reference-demotion|Phase 23]].
- **Fixture generator.** `scripts/generate_rust_fixtures.py` imports
  this package to produce the frozen `config_hashes.json`, codebook
  fixtures, quantize fixtures, and residual fixtures that live under
  `rust/crates/tinyquant-core/tests/fixtures/` in Git LFS.
- **Self-parity baseline.** The `tests/parity/` suite exercises the
  reference against itself (trivial tautology tests) as a sanity
  check that the fixtures are well-formed and the fixtures and
  oracle stay in sync as Phase 24 approaches.

## How to invoke it

```bash
# From the repo root, with pyproject's pythonpath in effect:
pytest                                # full 214-test suite
pytest --cov=tinyquant_py_reference   # with coverage gate
pytest -m parity -v                   # parity scaffold (rs skipped)

# Direct import (Python REPL from repo root):
python -c "import sys; sys.path.insert(0, 'tests/reference'); \
  import tinyquant_py_reference as ref; print(ref.codec.CodecConfig)"

# Fixture regeneration for the Rust suite:
python scripts/generate_rust_fixtures.py hashes
python scripts/generate_rust_fixtures.py codebook
```

## How it relates to Phase 24

Phase 24 reclaims the `tinyquant-cpu` PyPI name with a Rust-backed
fat wheel at version `0.2.0`. That wheel re-exports the same public
surface as `v0.1.1`, backed by the Rust implementation in
`rust/crates/tinyquant-py/`. When the fat wheel is installed into a
venv, the `rs` fixture in `tests/parity/conftest.py` stops skipping
and every cross-impl test in `tests/parity/test_cross_impl.py`
becomes a live byte-equality / allclose assertion that compares the
Rust-backed `tinyquant_cpu` against this reference.

Put another way:

- `0.1.x` line on PyPI = this package, distributed as a Python
  wheel. Frozen at `0.1.1`, not yanked.
- `0.2.x+` line on PyPI = the Rust-backed fat wheel that ships the
  same import path and public API. This package stays in-repo as
  its parity oracle.

## Contract with the Rust core

Anything documented under
[[design/rust/serialization-format|Serialization Format]],
[[design/rust/numerical-semantics|Numerical Semantics]], and
[[design/rust/testing-strategy|Testing Strategy]] §"Reference as
oracle" is satisfied by this reference. Changes that alter
byte-level output are **not** made to this package without a
coordinated Rust-side rollout.

## See also

- [[plans/rust/phase-23-python-reference-demotion|Phase 23: Python Reference Demotion]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/serialization-format|Serialization Format]]
- [[TinyQuant]]
