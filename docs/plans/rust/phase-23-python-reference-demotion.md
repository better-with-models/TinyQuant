---
title: "Phase 23: Python Reference Demotion"
tags:
  - plans
  - rust
  - phase-23
  - python
  - reference
  - testing
  - refactor
date-created: 2026-04-13
status: complete
category: planning
---

# Phase 23: Python Reference Demotion

> [!info] Goal
> Move the pure-Python implementation from a **shipped PyPI artifact**
> (`tinyquant-cpu`, currently `v0.1.1`) to a **test-only reference
> implementation** that lives alongside the test suite and is never
> installed by end users. The reference continues to exist as a
> per-commit differential oracle for the Rust core and the Phase 24
> fat wheel — nothing more. Its public surface is frozen at the
> `v0.1.1` behavior and its only consumers after this phase are the
> parity tests under `tests/parity/`.

> [!note] Prerequisites
> - Phase 22 complete: `tinyquant-py` PyO3 wheel published to PyPI
>   under the transitional name `tinyquant-rs`.
> - `rust-parity.yml` green for ≥ 7 consecutive days on `main`.
> - `COMPATIBILITY.md` up to date with the Phase 22 matrix.
> - The Rust core passes every byte-exact serialization round-trip
>   test against `tinyquant_cpu v0.1.1` (Phase 16 deliverable).

> [!abstract] Scope boundary
> This plan is **only** about the physical move and the wheel-leakage
> guard. Phase 24 is the plan that reclaims the `tinyquant-cpu` name
> on PyPI with a Rust-backed fat wheel. Phase 23 must leave the
> published `0.1.1` wheel alone and must not touch any Rust code.

## Deliverables

| Artifact | Change |
|---|---|
| `src/tinyquant_cpu/` | **Moved** to `tests/reference/tinyquant_py_reference/` via `git mv` (history preserved) |
| `pyproject.toml` | `[tool.hatch.build.targets.wheel].packages` cleared; `pythonpath` extended; coverage paths rewritten |
| `tests/conftest.py` | Import swapped from `tinyquant_cpu` to `tinyquant_py_reference`; every other test rewritten the same way |
| `tests/parity/__init__.py` | **New**, marker file for the parity suite |
| `tests/parity/conftest.py` | **New**, fixtures that expose the reference as `ref` and leave room for the Rust impl as `rs` (Phase 24) |
| `tests/parity/test_cross_impl.py` | **New**, parametrized cross-impl parity scaffold (self-parity only in Phase 23) |
| `.github/workflows/ci.yml` | **New job** `build-package-does-not-leak-reference` that fails on `tests/reference/*` in the built wheel |
| `docs/entities/python-reference-implementation.md` | **New** wiki page documenting the demoted status |
| `docs/entities/TinyQuant.md` | Updated status paragraph |
| `AGENTS.md`, `README.md`, `CLAUDE.md` | Updated language: reference implementation, not shipped |
| `CHANGELOG.md` | `[Unreleased]` entry documenting the refactor |

## Non-goals

- **No algorithm, format, or API changes.** The reference must remain
  byte-for-byte compatible with `tinyquant-cpu v0.1.1`. Anything that
  would make the reference disagree with the published `0.1.1` wheel is
  out of scope.
- **No PyPI publishing.** Phase 23 does not bump the version. The old
  `0.1.1` wheel stays on PyPI, untouched and unyanked. Phase 24 is what
  ships `0.2.0`.
- **No Rust changes.** The Rust crates, the `tinyquant-py` PyO3 wheel,
  and the `rust-parity.yml` workflow are all left as Phase 22 left them.
- **No removal of `numpy` from the reference.** The reference remains a
  self-contained Python package that can be imported in a dev shell. It
  simply does not ship in any wheel.
- **No deletion of the published `0.1.1` wheel.** It is left on PyPI so
  users pinned to it stay functional. A `Yanked: no` stance is explicit.

## Detailed file movement map

The move is mechanical: `git mv src/tinyquant_cpu
tests/reference/tinyquant_py_reference`. Internal imports within the
package are unchanged because the package continues to import itself
under its new top-level name. Every `from tinyquant_cpu.*` inside the
package becomes `from tinyquant_py_reference.*` via a single
`git ls-files -z | xargs -0 sed -i` sweep (see Step 2).

> [!note] Path renaming rule
> The Python package name changes from `tinyquant_cpu` to
> `tinyquant_py_reference`. The directory change is a straight rename.
> No files are added, removed, or merged inside the package.

### Package sources

| Before (tracked path) | After (tracked path) |
|---|---|
| `src/tinyquant_cpu/__init__.py` | `tests/reference/tinyquant_py_reference/__init__.py` |
| `src/tinyquant_cpu/_types.py` | `tests/reference/tinyquant_py_reference/_types.py` |
| `src/tinyquant_cpu/py.typed` | `tests/reference/tinyquant_py_reference/py.typed` |
| `src/tinyquant_cpu/backend/__init__.py` | `tests/reference/tinyquant_py_reference/backend/__init__.py` |
| `src/tinyquant_cpu/backend/brute_force.py` | `tests/reference/tinyquant_py_reference/backend/brute_force.py` |
| `src/tinyquant_cpu/backend/protocol.py` | `tests/reference/tinyquant_py_reference/backend/protocol.py` |
| `src/tinyquant_cpu/backend/adapters/__init__.py` | `tests/reference/tinyquant_py_reference/backend/adapters/__init__.py` |
| `src/tinyquant_cpu/backend/adapters/pgvector.py` | `tests/reference/tinyquant_py_reference/backend/adapters/pgvector.py` |
| `src/tinyquant_cpu/codec/__init__.py` | `tests/reference/tinyquant_py_reference/codec/__init__.py` |
| `src/tinyquant_cpu/codec/_errors.py` | `tests/reference/tinyquant_py_reference/codec/_errors.py` |
| `src/tinyquant_cpu/codec/_quantize.py` | `tests/reference/tinyquant_py_reference/codec/_quantize.py` |
| `src/tinyquant_cpu/codec/codebook.py` | `tests/reference/tinyquant_py_reference/codec/codebook.py` |
| `src/tinyquant_cpu/codec/codec.py` | `tests/reference/tinyquant_py_reference/codec/codec.py` |
| `src/tinyquant_cpu/codec/codec_config.py` | `tests/reference/tinyquant_py_reference/codec/codec_config.py` |
| `src/tinyquant_cpu/codec/compressed_vector.py` | `tests/reference/tinyquant_py_reference/codec/compressed_vector.py` |
| `src/tinyquant_cpu/codec/rotation_matrix.py` | `tests/reference/tinyquant_py_reference/codec/rotation_matrix.py` |
| `src/tinyquant_cpu/corpus/__init__.py` | `tests/reference/tinyquant_py_reference/corpus/__init__.py` |
| `src/tinyquant_cpu/corpus/compression_policy.py` | `tests/reference/tinyquant_py_reference/corpus/compression_policy.py` |
| `src/tinyquant_cpu/corpus/corpus.py` | `tests/reference/tinyquant_py_reference/corpus/corpus.py` |
| `src/tinyquant_cpu/corpus/events.py` | `tests/reference/tinyquant_py_reference/corpus/events.py` |
| `src/tinyquant_cpu/corpus/vector_entry.py` | `tests/reference/tinyquant_py_reference/corpus/vector_entry.py` |
| `src/tinyquant_cpu/tools/__init__.py` | `tests/reference/tinyquant_py_reference/tools/__init__.py` |
| `src/tinyquant_cpu/tools/dump_serialization.py` | `tests/reference/tinyquant_py_reference/tools/dump_serialization.py` |

After the move, `src/` is deleted. Hatchling is no longer pointed at it.
The `src/` directory does not come back in Phase 24 — Phase 24 uses the
Rust workspace layout (`rust/crates/tinyquant-py/python/...`) as its
Python source root.

### Test files that change content (imports only)

Every test file rewrites `tinyquant_cpu` → `tinyquant_py_reference`.
The files themselves do not move.

| Path | Imports touched |
|---|---|
| `tests/conftest.py` | 6 `from tinyquant_cpu...` lines |
| `tests/architecture/test_dependency_direction.py` | module-path allowlist |
| `tests/backend/test_brute_force.py` | 2 imports |
| `tests/codec/test_codebook.py` | 1 import |
| `tests/codec/test_codec.py` | 3 imports |
| `tests/codec/test_codec_config.py` | 1 import |
| `tests/codec/test_compressed_vector.py` | 2 imports |
| `tests/codec/test_rotation_matrix.py` | 1 import |
| `tests/corpus/test_compression_policy.py` | 1 import |
| `tests/corpus/test_corpus.py` | 3 imports |
| `tests/corpus/test_events.py` | 2 imports |
| `tests/corpus/test_vector_entry.py` | 1 import |
| `tests/calibration/conftest.py` | 4 imports |
| `tests/calibration/test_*.py` | module-level imports + `importlib` lookups |
| `tests/integration/conftest.py` | 3 imports |
| `tests/integration/test_codec_corpus.py` | 3 imports |
| `tests/integration/test_corpus_backend.py` | 3 imports |
| `tests/integration/test_pgvector.py` | 2 imports + DSN guard |
| `tests/integration/test_serialization.py` | 2 imports |
| `tests/e2e/test_full_pipeline.py` | 4 imports |
| `tests/test_smoke.py` | 1 import + `__version__` assertion |

The total is deterministic: `git grep -l tinyquant_cpu tests/ | wc -l`
on the pre-phase tip yields ~22 files. The sweep is a single
`sed` pass in Step 1. `ruff --fix` afterward normalizes import
ordering.

### New files created

| Path | Purpose |
|---|---|
| `tests/reference/__init__.py` | Empty marker — allows `pytest --collect-only` to descend without mistaking tests |
| `tests/reference/README.md` | One-screen explainer: "This is the test-only pure-Python reference. Do not package." |
| `tests/parity/__init__.py` | Empty marker |
| `tests/parity/conftest.py` | `ref` fixture (and a `pytest.skip` placeholder for `rs` until Phase 24) |
| `tests/parity/test_cross_impl.py` | Scaffold parity suite (described below) |
| `docs/entities/python-reference-implementation.md` | Wiki page |

## pyproject.toml changes

Three sections change: `[tool.hatch.build.targets.wheel]` (new,
restricts what the wheel contains), `[tool.pytest.ini_options]`
(extended to expose the reference on the import path), and the two
coverage tables (pointed at the new path, with `fail_under` rewritten
for the reference-as-reference contract).

### Before (relevant slices)

```toml
[project]
name = "tinyquant-cpu"
version = "0.1.1"
# ...

[tool.coverage.run]
parallel = true
source = ["tinyquant_cpu"]
omit = [
    "src/tinyquant_cpu/tools/*",
]

[tool.coverage.paths]
source = [
    "src/tinyquant_cpu",
    "*/src/tinyquant_cpu",
    "*/site-packages/tinyquant_cpu",
]

[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--import-mode=importlib -x --tb=short"
markers = [
    "pgvector: requires PostgreSQL with pgvector (PGVECTOR_TEST_DSN)",
]
```

### After

```toml
[project]
name = "tinyquant-cpu"
version = "0.1.1"
# unchanged: Phase 23 does NOT bump the version. Phase 24 does.
# No build-time artifacts from Phase 23 are published.

# Explicitly tell hatchling there are no package sources.
# Without this, hatchling tries to auto-detect a package under src/
# and fails the build entirely — which is actually the desired
# failure mode for Phase 23, because nothing should be built from
# this tree until Phase 24 introduces the Rust backend.
[tool.hatch.build.targets.wheel]
packages = []
bypass-selection = true

# Keep sdists buildable for local archival, but exclude the reference.
[tool.hatch.build.targets.sdist]
exclude = [
    "tests/reference/",
    "tests/parity/",
    "rust/",
    "experiments/",
    "scripts/",
    "docs/",
]

[tool.coverage.run]
parallel = true
# Coverage now measures the reference package, but the fail-under
# gate is set against the *parity* harness, not the reference
# itself (see tests/parity/pytest.ini fragment below).
source = ["tinyquant_py_reference"]
omit = [
    "tests/reference/tinyquant_py_reference/tools/*",
]

[tool.coverage.paths]
source = [
    "tests/reference/tinyquant_py_reference",
    "*/tests/reference/tinyquant_py_reference",
]

[tool.pytest.ini_options]
testpaths = ["tests"]
# pythonpath is how pytest adds tests/reference to sys.path at
# collection time. --import-mode=importlib is preserved so that
# tests/parity and tests/reference do not share a module cache.
pythonpath = ["tests/reference"]
addopts = "--import-mode=importlib -x --tb=short"
markers = [
    "pgvector: requires PostgreSQL with pgvector (PGVECTOR_TEST_DSN)",
    "parity: cross-implementation parity between Python reference and Rust",
]
```

> [!warning] Hatchling "packages = []" footgun
> Hatchling treats an empty `packages` list as "do not build anything".
> That is exactly what Phase 23 wants. But if a future agent runs
> `python -m build .` on this tree they will get an explicit
> `ValueError: Unable to determine which files to ship inside the wheel`.
> That error is the Phase 23 build contract: **no wheel is produced**.
> The CI guard (below) reinforces this by actively failing if a wheel
> ever *is* produced.

## CI workflow changes

A new job is added to `.github/workflows/ci.yml`:
`build-package-does-not-leak-reference`. It runs after the Python test
matrix. Its sole purpose is to catch any future regression where
someone re-adds the reference to the wheel.

```yaml
# .github/workflows/ci.yml  (excerpt — add after the pytest matrix)

  build-package-does-not-leak-reference:
    name: Wheel leakage guard
    needs: [test-python]   # only run if the reference suite passes
    runs-on: ${{ fromJSON(needs.choose-runner.outputs.runner) }}
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: ${{ env.PYTHON_VERSION }}

      - name: Install build tooling
        run: python -m pip install --upgrade build==1.* hatchling

      # Phase 23 intentionally produces NO wheel. `python -m build` is
      # expected to fail with a hatchling "no packages" error. If it
      # succeeds, that is itself a regression (someone re-enabled the
      # wheel target), so this step fails on *success* as well as on
      # unexpected leakage.
      - name: Build must fail (Phase 23 contract)
        id: build
        continue-on-error: true
        run: python -m build --wheel --outdir dist/

      - name: Assert that hatchling refused to build
        if: steps.build.outcome == 'success'
        run: |
          echo "ERROR: python -m build unexpectedly produced a wheel."
          echo "Phase 23 contract is that no wheel is produced from this tree."
          echo "Contents of dist/:"
          ls -la dist/
          exit 1

      # Defensive second layer: if a future PR intentionally re-enables
      # the wheel target (e.g. Phase 24 landing in the same branch),
      # inspect whatever was produced and fail on any reference path.
      - name: Inspect any wheel that was produced (defense in depth)
        if: steps.build.outcome == 'success'
        run: |
          set -euo pipefail
          for whl in dist/*.whl; do
            echo "=== Inspecting $whl ==="
            # `unzip -l` prints the wheel manifest. A leaked reference
            # would show as `tests/reference/...` or
            # `tinyquant_py_reference/...` inside the archive.
            if unzip -l "$whl" | grep -E '(^|/)(tests/reference|tinyquant_py_reference)(/|$)'; then
              echo "LEAK: wheel $whl contains reference-implementation files."
              exit 1
            fi
          done
```

### Why two layers

1. The **primary** assertion is that Phase 23's `pyproject.toml` causes
   `python -m build` to fail cleanly. That is the strong contract: no
   wheel exists at all, so no wheel can leak.
2. The **secondary** `unzip -l | grep` assertion is a belt-and-braces
   check that continues to be meaningful in Phase 24 (when a wheel
   legitimately starts being produced again). Phase 24 keeps this job
   and reuses it unchanged.

### Test matrix update

The existing `test-python` job keeps its matrix. One line changes:

```yaml
# before
- run: pip install -e ".[dev]"
# after
- run: |
    python -m pip install --upgrade pip
    python -m pip install numpy==1.26.* pytest==8.* pytest-cov==5.* hypothesis==6.* ruff==0.5.* mypy==1.10.*
```

The tree no longer has a buildable package so `pip install -e .` is not
meaningful. Dev dependencies are installed directly. Pinning matches
what `[project.optional-dependencies].dev` resolved to at `0.1.1` tag
time, so the reference remains reproducible.

## Cross-impl parity harness

The harness scaffold lives at `tests/parity/`. In Phase 23 it exists,
it is discoverable, and it passes trivially (the reference is compared
against itself — every assertion is a tautology). Phase 24 populates
the `rs` side and the scaffold lights up as a real parity suite.

### `tests/parity/conftest.py`

```python
"""Cross-implementation parity fixtures.

Phase 23: only the pure-Python reference is wired in. The ``rs``
fixture raises ``pytest.skip`` until Phase 24 installs the Rust-backed
``tinyquant_cpu`` fat wheel into the venv.

Phase 24 will replace the skip with::

    import tinyquant_cpu as rs   # the Rust-backed fat wheel

and every parity test becomes a live assertion.
"""
from __future__ import annotations

from collections.abc import Iterator
from typing import Any

import numpy as np
import pytest

import tinyquant_py_reference as ref_pkg


@pytest.fixture(scope="session")
def ref() -> Any:
    """The pure-Python reference implementation. Always available."""
    return ref_pkg


@pytest.fixture(scope="session")
def rs() -> Iterator[Any]:
    """The Rust-backed implementation. Phase 24 wires this up."""
    try:
        import tinyquant_cpu as rs_pkg  # noqa: PLC0415
    except ImportError:
        pytest.skip(
            "Rust-backed tinyquant_cpu not installed; "
            "parity tests require Phase 24 fat wheel."
        )
    else:
        yield rs_pkg


@pytest.fixture(params=[(4, 42, 64), (2, 0, 128), (8, 999, 256)])
def cfg_triplet(request: pytest.FixtureRequest) -> tuple[int, int, int]:
    """(bit_width, seed, dimension) tuples covering every bit-width."""
    return request.param


@pytest.fixture()
def rng(cfg_triplet: tuple[int, int, int]) -> np.random.Generator:
    _, seed, _ = cfg_triplet
    return np.random.default_rng(seed)


@pytest.fixture()
def vector(rng: np.random.Generator, cfg_triplet: tuple[int, int, int]):
    _, _, dim = cfg_triplet
    return rng.standard_normal(dim).astype(np.float32)


@pytest.fixture()
def batch(rng: np.random.Generator, cfg_triplet: tuple[int, int, int]):
    _, _, dim = cfg_triplet
    return rng.standard_normal((128, dim)).astype(np.float32)
```

### `tests/parity/test_cross_impl.py`

```python
"""Cross-implementation parity scaffold.

Phase 23 deliverable: structure exists, runs green against the
reference alone (trivial self-parity), and Phase 24 flips the
``rs`` fixture on to make every test meaningful.
"""
from __future__ import annotations

import numpy as np
import pytest


pytestmark = pytest.mark.parity


class TestConfigParity:
    def test_config_hash_matches_self(self, ref, cfg_triplet):
        bw, seed, dim = cfg_triplet
        a = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        b = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        assert a.config_hash == b.config_hash

    def test_config_hash_cross_impl(self, ref, rs, cfg_triplet):
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        assert py_cfg.config_hash == rs_cfg.config_hash


class TestRotationParity:
    def test_rotation_deterministic(self, ref, cfg_triplet, vector):
        bw, seed, dim = cfg_triplet
        r1 = ref.codec.RotationMatrix(seed=seed, dimension=dim)
        r2 = ref.codec.RotationMatrix(seed=seed, dimension=dim)
        np.testing.assert_array_equal(r1.apply(vector), r2.apply(vector))

    def test_rotation_cross_impl(self, ref, rs, cfg_triplet, vector):
        bw, seed, dim = cfg_triplet
        py_rot = ref.codec.RotationMatrix(seed=seed, dimension=dim)
        rs_rot = rs.codec.RotationMatrix(seed=seed, dimension=dim)
        np.testing.assert_allclose(
            py_rot.apply(vector), rs_rot.apply(vector), atol=1e-6
        )


class TestCompressRoundTrip:
    def test_self_round_trip(self, ref, cfg_triplet, batch):
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb = ref.codec.Codebook.train(batch, cfg)
        codec = ref.codec.Codec(cfg, cb)
        cvs = [codec.compress(row) for row in batch]
        recon = np.stack([codec.decompress(cv) for cv in cvs])
        # Sanity bound only — tight MSE is a calibration test, not parity.
        assert float(np.mean((recon - batch) ** 2)) < 1.0

    def test_cross_impl_round_trip(self, ref, rs, cfg_triplet, batch):
        """Compress with reference, decompress with Rust and vice versa."""
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        py_cb = ref.codec.Codebook.train(batch, py_cfg)
        py_codec = ref.codec.Codec(py_cfg, py_cb)

        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cb = rs.codec.Codebook(entries=py_cb.entries, bit_width=bw)
        rs_codec = rs.codec.Codec(rs_cfg, rs_cb)

        for row in batch[:8]:
            py_cv = py_codec.compress(row)
            rs_cv = rs.codec.CompressedVector.from_bytes(py_cv.to_bytes())
            np.testing.assert_allclose(
                py_codec.decompress(py_cv),
                rs_codec.decompress(rs_cv),
                atol=1e-3,
            )


class TestSerializationParity:
    def test_codebook_bytes_stable(self, ref, cfg_triplet, batch):
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb = ref.codec.Codebook.train(batch, cfg)
        assert cb.to_bytes() == cb.to_bytes()

    def test_codebook_cross_impl_bytes(self, ref, rs, cfg_triplet, batch):
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb = ref.codec.Codebook.train(batch, cfg)
        rs_cb = rs.codec.Codebook.from_bytes(cb.to_bytes())
        assert rs_cb.to_bytes() == cb.to_bytes()
```

### Matrix coverage

| Dimension | Values in Phase 23 | Values Phase 24 expands to |
|---|---|---|
| `bit_width` | `{2, 4, 8}` | `{2, 4, 8}` |
| `seed` | `{0, 42, 999}` | add property-based Hypothesis sweep |
| `dimension` | `{64, 128, 256}` | `{64, 128, 256, 768, 1536}` |
| Batch sizes | `{1, 128}` | `{1, 16, 128, 4096}` |
| Formats covered | `CodecConfig`, `RotationMatrix`, `Codec`, `Codebook`, `CompressedVector` | add `Corpus`, `Policy`, `BruteForceBackend` |

In Phase 23, every `*_cross_impl` test is skipped at the `rs` fixture
level — the suite turns green because the self-parity tests are
meaningful and the cross-impl ones are not collected as failures. The
`pytest.mark.parity` marker allows `pytest -m "not parity"` to run the
reference-only tree (same 214 tests as today, modulo the rename).

## Versioning strategy

> [!warning] No PyPI publish in Phase 23
> Phase 23 is a **pure refactor commit**. The version string in
> `pyproject.toml` stays at `0.1.1`. Nothing is uploaded to PyPI or
> TestPyPI. The `release.yml` workflow is not triggered — tags are not
> created.

**What stays on PyPI after Phase 23 merges:**

- `tinyquant-cpu==0.1.1` — the last pure-Python wheel. Left alone.
  Users with `tinyquant-cpu==0.1.1` in `requirements.txt` keep working.
- No new pre-release.
- No `0.1.2`.

**What Phase 24 does** (for context, not this plan's deliverable):

- Phase 24 bumps the version to `0.2.0` and starts shipping a
  Rust-backed fat wheel under the same PyPI name.
- `0.2.0` is advertised as a drop-in replacement for `0.1.1` — same
  import path, same public API surface, different engine room.
- The `0.1.x` line stays as-is. It is not yanked. A CHANGELOG note
  clarifies that `0.1.x` is the pure-Python implementation and `0.2.x+`
  is the Rust implementation.

**Git tag policy for Phase 23:** none. The commit lands on `main`
behind a PR titled `phase-23: demote pure-python to reference`. The
next release tag the repo will see is `v0.2.0-rc.1` cut by Phase 24.

## Steps (TDD order)

### Step 1: Red — rewrite test imports without moving code

> [!note] Red state
> After Step 1 runs in isolation, the test suite does not collect
> because `tinyquant_py_reference` does not yet exist. The red is
> expected and the Step 2 move turns it green.

```bash
# From repo root. On Windows use git-bash / WSL for sed.
git checkout -b phase-23-python-reference-demotion
git grep -l 'tinyquant_cpu' -- 'tests/**/*.py' 'tests/*.py' \
  | xargs sed -i 's/tinyquant_cpu/tinyquant_py_reference/g'

# Sanity: nothing outside tests/ was touched.
git diff --stat -- 'src/' 'rust/' 'scripts/' 'docs/'   # must be empty

# Confirm the red: collection fails with ImportError.
pytest --collect-only 2>&1 | tee /tmp/phase23-step1-red.log
grep -q 'ModuleNotFoundError.*tinyquant_py_reference' /tmp/phase23-step1-red.log
```

Commit the red snapshot so the diff is reviewable:

```bash
git add tests/
git commit -m "phase-23 step 1: rewrite test imports (red — pkg not yet moved)"
```

### Step 2: Move the package (`git mv`)

```bash
# Create the parent directory.
mkdir -p tests/reference
git mv src/tinyquant_cpu tests/reference/tinyquant_py_reference

# Rewrite internal imports inside the package. This is a *different*
# sed pass from Step 1: this one rewrites source, not tests.
git grep -l 'tinyquant_cpu' -- 'tests/reference/**/*.py' \
  | xargs sed -i 's/tinyquant_cpu/tinyquant_py_reference/g'

# Remove the now-empty src/ tree.
rmdir src/ 2>/dev/null || true

# Drop a README so nobody wonders why the reference lives under tests/.
cat > tests/reference/README.md <<'EOF'
# tinyquant_py_reference

The pure-Python reference implementation of TinyQuant. It lives here,
under `tests/`, to make clear it is **not a shipped artifact**. It
exists only as a differential oracle for the Rust core and the fat
wheel. See [phase-23 plan](../../docs/plans/rust/phase-23-python-reference-demotion.md).
EOF

git add tests/reference/README.md
git commit -m "phase-23 step 2: move tinyquant_cpu → tests/reference/tinyquant_py_reference"
```

### Step 3: Green — update pyproject, run full test suite

Apply the `pyproject.toml` diff from the section above (pythonpath,
coverage paths, `[tool.hatch.build.targets.wheel].packages = []`,
exclude tests/reference from sdist).

```bash
# Full suite — expect 214 passes, same count as v0.1.1.
pytest -q

# Coverage still gated — reference coverage is now what is measured.
pytest --cov=tinyquant_py_reference --cov-fail-under=90 -q
```

Refactor notes in this step:

- `tests/conftest.py` — verify that all fixtures import cleanly under
  the new name.
- `tests/architecture/test_dependency_direction.py` — the allowlist of
  allowed module prefixes changes from `tinyquant_cpu.*` to
  `tinyquant_py_reference.*`. Everything else about that test is
  unchanged.
- Commit: `phase-23 step 3: green — pyproject + pythonpath updated`.

### Step 4: Dead-wheel guard

Add the `build-package-does-not-leak-reference` job to
`.github/workflows/ci.yml` as specified in the **CI workflow changes**
section. Verify locally with:

```bash
# Phase 23 contract: build must FAIL (no packages).
python -m build --wheel --outdir dist/ 2>&1 | tee /tmp/phase23-build.log
grep -q 'Unable to determine which files to ship' /tmp/phase23-build.log \
  || grep -q 'no packages' /tmp/phase23-build.log \
  || { echo "Build succeeded — this is a Phase 23 regression."; exit 1; }
```

Commit: `phase-23 step 4: CI guard against reference leakage into wheels`.

### Step 5: Parity harness scaffold

Create `tests/parity/` with the three files shown in the
**Cross-impl parity harness** section. In Phase 23, the `rs` fixture
skips, so `pytest tests/parity/` yields: self-parity tests pass, cross-impl
tests skip.

```bash
pytest tests/parity/ -v
# expect: TestConfigParity::test_config_hash_matches_self -- passed
#         TestConfigParity::test_config_hash_cross_impl   -- SKIPPED (...)
#         ...
```

Commit: `phase-23 step 5: parity harness scaffold (rs skipped until phase 24)`.

### Step 6: Docs

Create and update:

- `docs/entities/python-reference-implementation.md` — new wiki page.
  Frontmatter:

  ```yaml
  ---
  title: "Python Reference Implementation"
  tags: [entities, python, reference, testing]
  date-created: 2026-04-13
  status: active
  category: testing
  ---
  ```

  Body: what it is, where it lives, why it exists, how to invoke it,
  how it relates to Phase 24's Rust-backed `tinyquant_cpu`. Links to
  [[plans/rust/phase-23-python-reference-demotion|Phase 23]].

- `docs/entities/TinyQuant.md` — status paragraph updated to reflect
  that the Python implementation is now reference-only.
- `AGENTS.md` — the "Repository overview" and "Key directories" tables
  updated: `src/` is gone; `tests/reference/` is noted.
- `README.md` — installation section replaces `pip install tinyquant-cpu`
  with a note that `0.1.1` is the last pure-Python release and `0.2.0+`
  (Phase 24) ships the Rust-backed wheel under the same name. Dev
  setup instructions show `pytest` and `tests/reference/`.
- `CLAUDE.md` — same language changes as AGENTS.md (they duplicate
  content).
- `CHANGELOG.md` — `[Unreleased]` section:

  ```markdown
  ## [Unreleased]

  ### Changed
  - Pure-Python implementation demoted to a test-only reference at
    `tests/reference/tinyquant_py_reference/`. It is no longer shipped
    on PyPI. The last pure-Python release remains `tinyquant-cpu==0.1.1`.
    Phase 24 will reclaim the `tinyquant-cpu` name with a Rust-backed
    fat wheel at version `0.2.0`.
  ```

Commit: `phase-23 step 6: docs — reference-only status`.

## Docs updates checklist

Hunt performed via `git grep -l 'tinyquant_cpu\|tinyquant-cpu'` on the
pre-phase tip; every hit below must be evaluated. Not every occurrence
changes — Phase 24 context (Rust-backed `tinyquant_cpu` on PyPI) means
some `tinyquant_cpu` references remain *valid*, they just now refer to
the Rust wheel, not the Python source. The checklist flags each.

| File | Action |
|---|---|
| `README.md` | **Update.** Replace install banner and dev-setup section (see Step 6). |
| `.github/README.md` | **Update.** Same substance as `README.md`. |
| `AGENTS.md` | **Update.** `src/` → `tests/reference/` in the Key directories table. |
| `CLAUDE.md` | **Update.** Mirror AGENTS.md changes. |
| `CHANGELOG.md` | **Append** `[Unreleased]` section (Step 6). |
| `docs/entities/TinyQuant.md` | **Update.** Status paragraph: reference-only Python. |
| `docs/entities/python-reference-implementation.md` | **Create.** New page. |
| `docs/roadmap.md` | **Update.** Phase 23 entry moves to "in-progress" → "complete". |
| `docs/log.md` | **Append** the Phase 23 operation entry. |
| `docs/plans/phase-05-backend-layer.md` | **No-op.** Historical plan; do not rewrite. |
| `docs/plans/rust/phase-12-shared-types-and-errors.md` | **No-op.** Historical. |
| `docs/plans/rust/phase-13-rotation-numerics.md` | **No-op.** |
| `docs/plans/rust/phase-16-serialization-parity.md` | **No-op.** |
| `docs/plans/rust/phase-18-corpus-aggregate.md` | **No-op.** |
| `docs/plans/rust/phase-19-brute-force-pgvector.md` | **No-op.** |
| `docs/plans/rust/phase-22-pyo3-cabi-release.md` | **No-op.** Phase 22 predates the rename. |
| `docs/plans/rust/phase-24-python-fat-wheel-official.md` | **Owned by another agent.** Do not touch. |
| `docs/plans/rust/phase-25-typescript-npm-package.md` | **Owned by another agent.** Do not touch. |
| `docs/design/rust/ffi-and-bindings.md` | **Review.** Confirm every `tinyquant_cpu` reference means Phase-24 Rust wheel, not Python source. Add a callout if ambiguous. |
| `docs/design/rust/release-strategy.md` | **Review.** Same as above. |
| `docs/design/rust/goals-and-non-goals.md` | **Review.** |
| `docs/design/rust/type-mapping.md` | **Review.** |
| `docs/design/rust/testing-strategy.md` | **Update.** New section on reference-as-oracle. |
| `docs/design/rust/phase-13-implementation-notes.md` | **No-op.** |
| `docs/design/rust/phase-14-implementation-notes.md` | **No-op.** |
| `docs/design/rust/README.md` | **Review.** |
| `docs/design/rust/crate-topology.md` | **No-op.** |
| `docs/design/rust/benchmark-harness.md` | **No-op.** |
| `docs/design/rust/serialization-format.md` | **No-op.** |
| `docs/design/rust/error-model.md` | **No-op.** |
| `docs/classes/codebook.md` | **Review.** Paths may need `tests/reference/` prefix. |
| `docs/CI-plan/workflow-definition.md` | **Update.** Mention new `build-package-does-not-leak-reference` job. |
| `docs/CD-plan/release-workflow.md` | **Update.** Clarify no Phase-23 release exists. |
| `docs/qa/unit-tests/README.md` | **Update.** Reference lives under `tests/reference/` now. |
| `docs/research/python-rust-wrapper-distribution.md` | **No-op.** Research, not wiki. |
| `docs/superpowers/plans/*.md` | **No-op.** Historical. |
| `docs/research/tinyquant-research/README.md` | **No-op.** |
| `scripts/ci_local_simulate.sh` | **Update.** Simulator must exercise the new wheel-leak job. |
| `scripts/generate_rust_fixtures.py` | **Update.** Import path `tinyquant_cpu` → `tinyquant_py_reference`. |
| `src/tinyquant_cpu/tools/dump_serialization.py` | **Moves** in Step 2 along with the rest of the package. |
| `rust/crates/tinyquant-core/**/*.rs` | **Review comments/docstrings only.** Rust code referring to "tinyquant_cpu" as a behavior oracle is still correct; no code change needed. |
| `rust/crates/tinyquant-py/src/lib.rs` | **No-op.** Phase 22 concern. |

## Acceptance criteria

Every bullet is machine-verifiable or a single grep away from being so.

1. `pip install tinyquant-cpu==0.1.1` still succeeds from PyPI and the
   resulting environment `import tinyquant_cpu` works. (Verified by a
   CI matrix job that pins to `0.1.1`.)
2. `python -m build .` on the Phase 23 tip **fails** with an explicit
   hatchling "no packages" error. No `.whl` appears in `dist/`.
3. `pytest tests/ -q` runs and passes 214 tests (same count as
   pre-phase) with the reference under its new name.
4. `pytest tests/parity/ -v -m parity` collects and runs; self-parity
   tests pass; cross-impl tests are `SKIPPED` with the message
   `Rust-backed tinyquant_cpu not installed`.
5. `grep -R 'tinyquant_cpu' tests/` returns zero matches. (All test
   imports are rewritten.)
6. `grep -R 'tinyquant_cpu' src/` returns zero matches. (Because `src/`
   is gone.)
7. CI job `build-package-does-not-leak-reference` is green on a clean
   Phase 23 branch and red if a follow-up PR re-introduces a wheel
   that contains `tests/reference/` or `tinyquant_py_reference/`.
8. `pytest --cov=tinyquant_py_reference --cov-fail-under=90` passes.
   Coverage number is within 1% of the `v0.1.1` baseline of 90.95%.
9. `docs/entities/python-reference-implementation.md` exists, has
   valid frontmatter, and is linked from `docs/entities/TinyQuant.md`.
10. `git log --follow` on any moved file still traces through the
    rename — history is preserved.

## Risks

### Risk 1: Consumers importing private modules break silently

**Severity:** low · **Likelihood:** medium

Anyone reaching into `tinyquant_cpu._quantize`, `tinyquant_cpu._types`,
or `tinyquant_cpu.codec._errors` (underscore-prefixed, not in `__all__`)
is technically using private API and is not covered by our
compatibility contract. Phase 24 ships the Rust wheel that re-exports
only the public surface; anyone using private modules hits an
`ImportError`.

**Mitigation:**
- The `0.1.x` line stays available on PyPI, **not yanked**. Users who
  depend on private internals keep a working artifact.
- `CHANGELOG.md` explicitly calls out that `0.2.0` is a drop-in
  replacement only for the public API documented in the `0.1.1`
  README, not for private modules.
- GitHub Release notes for `0.2.0` (Phase 24) link to a migration FAQ.

### Risk 2: Coverage regressions / artificial inflation

**Severity:** medium · **Likelihood:** medium

Moving 100% of the production code out of `src/` into `tests/` changes
what `coverage.py` considers the "source under test". A naive
configuration could either drop the gate entirely (reference lives in
`tests/`, tests aren't normally their own coverage source) or double-
count (the reference IS the test, which artificially pumps coverage).

**Mitigation:**
- Explicit `[tool.coverage.run].source = ["tinyquant_py_reference"]`
  with a matching `[tool.coverage.paths].source`. No wildcard.
- `--cov-fail-under=90` gate stays on the reference.
- Phase 24 adds a **separate** coverage gate on the Rust-backed wheel
  (measured via the Phase 22 `tinyquant-py` PyO3 extension's Python
  shim). The two gates do not share a floor number.

### Risk 3: Future PR accidentally re-enables the wheel

**Severity:** high · **Likelihood:** low

If a well-meaning future contributor "fixes" the hatchling build error
by re-pointing `packages` at `tests/reference/tinyquant_py_reference`,
the reference would quietly be shipped again under the `tinyquant-cpu`
name — silently superseding the Phase 24 Rust wheel with a pure-Python
one.

**Mitigation:**
- The CI job `build-package-does-not-leak-reference` fails the build
  on any wheel that contains `tests/reference/` or
  `tinyquant_py_reference/`.
- An inline comment in `pyproject.toml` above `[tool.hatch.build.targets.wheel]`
  explains the "packages = []" contract and links to this plan.
- The Phase 24 plan, when it lands, keeps this guard and extends it to
  also fail on the reference appearing in the Rust-backed wheel.

### Risk 4: Parity harness drifts during the Phase 23 → Phase 24 gap

**Severity:** low · **Likelihood:** medium

Phase 23 introduces the `tests/parity/` suite with the `rs` fixture
skipped. If Phase 24 is delayed, the suite remains mostly skipped and
could rot (fixtures go stale, tests never exercise the skipped branch).

**Mitigation:**
- Self-parity tests are always live — they exercise the reference
  against itself and catch fixture-level bugs.
- The harness is deliberately thin in Phase 23 so that maintenance
  cost is minimal.
- A `TODO(phase-24)` comment at the top of `tests/parity/conftest.py`
  calls out the expected evolution.

### Risk 5: Editable install of dev tools breaks when `src/` is gone

**Severity:** low · **Likelihood:** low

Contributors who had `pip install -e .[dev]` in a venv before Phase 23
will find that a subsequent `pip install -e .[dev]` fails because
there is no package to install editably.

**Mitigation:**
- Updated `README.md` dev-setup section tells contributors to install
  bare dev deps (`numpy pytest pytest-cov hypothesis ruff mypy`)
  without the `-e .` flag.
- `pytest` continues to work because `tests/reference/` is on
  `pythonpath` via the `pyproject.toml` pytest config.

## Rollback plan

Phase 23 is a single refactor commit (logically; realistically 6 small
commits for reviewability). Rollback is clean at both the repo layer
and the PyPI layer.

**Git-level rollback:**

```bash
# If Phase 23 lands behind a single squash-merged PR:
git revert -m 1 <phase-23-merge-sha>

# If Phase 23 landed as a chain of 6 commits:
git revert --no-commit <step6-sha>..<step1-sha>
git commit -m "revert: phase-23 python reference demotion"
```

After the revert, `src/tinyquant_cpu/` is restored with its history
intact, the wheel-leakage CI job is removed, and `pyproject.toml` is
back to the shipped-wheel configuration. `pytest tests/` runs green
with the old imports.

**PyPI-level rollback:** no-op. Phase 23 publishes nothing; there is
nothing on PyPI to unpublish or yank. The `0.1.1` wheel is already
published and stays published regardless of whether Phase 23 is reverted.

**Downstream-consumer impact of rollback:** none. Both states of the
repo produce the same published `0.1.1` wheel behavior; the difference
is purely internal (source location + packaging config).

## See also

- [[plans/rust/phase-22-pyo3-cabi-release|Phase 22: Pyo3, C ABI, Release]]
- [[plans/rust/phase-24-python-fat-wheel-official|Phase 24: Python Fat-Wheel Official]]
- [[research/python-rust-wrapper-distribution|Distribution Research]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/release-strategy|Release and Versioning]]
- [[entities/python-reference-implementation|Python Reference Implementation]]
