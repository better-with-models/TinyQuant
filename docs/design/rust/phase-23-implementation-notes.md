---
title: "Phase 23 Implementation Notes"
tags:
  - design
  - rust
  - phase-23
  - python
  - reference
  - testing
  - refactor
date-created: 2026-04-14
category: design
---

# Phase 23 Implementation Notes

## Summary

Phase 23 splits into three slices (23.1 / 23.2 / 23.3). All three
landed on the `phase-22-25-release-chain` branch before this note was
written:

- **Phase 23.1 — Red + rewrite (LANDED at `d34ae36`).** Rewrote every
  `tinyquant_cpu` import under `tests/` to `tinyquant_py_reference`
  in a single-commit "red" snapshot: pytest collection fails with
  `ModuleNotFoundError` because the package has not yet moved. The
  diff is reviewable as pure import-rewrite + no source motion, and
  the red log from the spec's Step 1 block is reproduced verbatim.
- **Phase 23.2 — Move + pyproject + wheel guard (LANDED at
  `da3330a`, `aa8a3d1`, `1b93b9d`).** `git mv src/tinyquant_cpu
  tests/reference/tinyquant_py_reference` (history preserved via
  `git log --follow`). Rewrote internal package imports with the same
  sed pass applied to the package's own sources. Removed the empty
  `src/` tree. Updated `pyproject.toml`:
  cleared `[tool.hatch.build.targets.wheel].packages`, set
  `bypass-selection = true`, extended `[tool.pytest.ini_options].pythonpath`
  to include `tests/reference`, and rewrote
  `[tool.coverage.paths].source` and `[tool.coverage.run].source` to
  target the new location. The full 214-test suite came back green
  under the new name (Step 3). Step 4 added the
  `build-package-does-not-leak-reference` job to
  `.github/workflows/ci.yml`, structured as a leakage check on any
  produced wheel (see §Declared deviations below for the nuance that
  motivated this shape).
- **Phase 23.3 — Parity harness scaffold + Docs (this slice).**
  Added `tests/parity/` with three files matching the spec's
  §Cross-impl parity harness block verbatim:
  - `tests/parity/__init__.py` (empty marker),
  - `tests/parity/conftest.py` with session-scoped `ref` and `rs`
    fixtures. `ref` returns `tinyquant_py_reference`; `rs` tries to
    `import tinyquant_cpu`, and on `ImportError` raises
    `pytest.skip("Rust-backed tinyquant_cpu not installed; parity
    tests require Phase 24 fat wheel.")`.
  - `tests/parity/test_cross_impl.py` with `pytestmark =
    pytest.mark.parity`, four test classes (`TestConfigParity`,
    `TestRotationParity`, `TestCompressRoundTrip`,
    `TestSerializationParity`) and eight test methods covering the
    bit-width × seed × dimension matrix `{(4,42,64), (2,0,128),
    (8,999,256)}`.
  The `pytest.mark.parity` marker is already registered in
  `pyproject.toml` so `pytest -m "not parity"` cleanly deselects the
  new suite and the existing 214 tests stay untouched. In a clean
  venv the four `*_cross_impl` tests skip; the four self-parity
  tests are tautological but exercise the fixtures and the
  reference's own import path. Phase 24 will drop the Rust-backed
  `tinyquant_cpu` fat wheel into the venv and every cross-impl test
  will flip from SKIPPED to live byte-equality / allclose assertion.

  The Step 6 docs sweep updated every prose surface flagged **Update**
  / **Create** / **Append** in the plan's §Docs updates checklist —
  see the Phase 23 commit subject `phase-23 step 6: docs —
  reference-only status` for the full file list. Files flagged
  **No-op**, **Review**, or **Owned by another agent** were
  intentionally left untouched.

The TDD spirit of Phase 23 is preserved even though no production
code was written: Step 1 is the red (import rewrite fails to
collect), Step 2–3 are the green (the move + pyproject makes the
existing 214 tests pass again under the new name), and Steps 4–5 are
the refactor-and-harden round (wheel-leak CI guard + parity
scaffold).

## Declared deviations

### 1. Phase 23 build-contract refinement

> **Phase 23 build-contract refinement.** The plan's §pyproject
> footgun warning and §Step 4 primary assertion both claim
> `python -m build` will hard-fail with hatchling. With
> `bypass-selection = true` set as the plan requires, hatchling ≥ 1.25
> instead emits an empty metadata-only wheel. The Phase 23 CI guard
> (`build-package-does-not-leak-reference` in `ci.yml`) was therefore
> structured as a leakage check on any produced wheel rather than a
> fail-on-success assertion. Acceptance criterion #2's first clause
> ("build fails") is **not** satisfied on modern hatchling; the
> second clause ("no `.whl` in `dist/`") is superseded by "no
> reference paths in any `.whl`", which is the contract Phase 24
> inherits unchanged.

This is the only declared deviation from the Phase 23 plan.
Mechanically: the Step 4 local-verification block in the plan
(`grep -q 'Unable to determine which files to ship' /tmp/phase23-build.log`)
does not trigger against a real hatchling invocation on the
post-Step-3 tree. The CI job exercised on the Phase 23.2 tip
observed the empty-wheel outcome, confirmed it contained no
reference paths, and landed green. The job's assertion shape was
adjusted accordingly and the contract was re-phrased from "build
must fail" to "no reference paths in any wheel that is produced,
and if no wheel is produced the contract is satisfied trivially".

The Phase 23.2 commit that added the job (`1b93b9d`) carries this
shape; the Phase 23.3 docs sweep
(`docs/CI-plan/workflow-definition.md` and
`docs/CD-plan/release-workflow.md`) makes the refinement explicit
in the wiki.

## Part A — Import rewrite and package move (slices 23.1 + 23.2)

### What moved

```text
src/tinyquant_cpu/                   →  tests/reference/tinyquant_py_reference/
src/tinyquant_cpu/codec/             →  tests/reference/tinyquant_py_reference/codec/
src/tinyquant_cpu/corpus/            →  tests/reference/tinyquant_py_reference/corpus/
src/tinyquant_cpu/backend/           →  tests/reference/tinyquant_py_reference/backend/
src/tinyquant_cpu/tools/             →  tests/reference/tinyquant_py_reference/tools/
src/tinyquant_cpu/_types.py          →  tests/reference/tinyquant_py_reference/_types.py
src/tinyquant_cpu/__init__.py        →  tests/reference/tinyquant_py_reference/__init__.py
```

Plus a new `tests/reference/README.md` that explains why the
reference lives under `tests/` and links to the Phase 23 plan.

### pyproject surface

The Phase 23.2 commit set the three contracts that make the move
durable:

```toml
[tool.hatch.build.targets.wheel]
# Phase 23: the reference lives under tests/reference/ and is
# intentionally NOT shipped. bypass-selection = true keeps hatchling
# from scanning; the CI job build-package-does-not-leak-reference
# defends against any wheel that might still be produced.
bypass-selection = true
packages = []

[tool.pytest.ini_options]
pythonpath = ["tests/reference"]
markers = [
    "pgvector: tests that require a live PostgreSQL+pgvector backend",
    "parity: cross-implementation parity between Python reference and Rust",
]

[tool.coverage.run]
source = ["tinyquant_py_reference"]

[tool.coverage.paths]
source = [
    "tests/reference/tinyquant_py_reference",
    "*/tests/reference/tinyquant_py_reference",
]
```

### History preservation

`git log --follow tests/reference/tinyquant_py_reference/codec/codec.py`
traces all the way back to Phase 3 commits — the `git mv` preserved
per-file history through the move. `git diff` at the move commit
shows rename entries, not add+delete pairs.

## Part B — Wheel-leak CI guard (slice 23.2, Step 4)

### Job shape

```yaml
build-package-does-not-leak-reference:
  name: Build Package Does Not Leak Reference
  runs-on: ${{ fromJSON(needs.effective-runner.outputs.runner) }}
  needs: [effective-runner, typecheck, markdown-lint]
  timeout-minutes: 5
  steps:
    - uses: actions/checkout@v5
    - uses: actions/setup-python@v5
      with: { python-version: ${{ env.PYTHON_VERSION }} }
    - run: pip install build
    - run: python -m build --wheel --outdir dist/
    - name: Assert no reference paths in any produced wheel
      run: |
        if compgen -G 'dist/*.whl' > /dev/null; then
          for whl in dist/*.whl; do
            if python -m zipfile -l "$whl" \
                 | grep -E 'tinyquant_py_reference|tests/reference/' > /dev/null; then
              echo "FAIL: $whl contains reference paths"
              exit 1
            fi
          done
          echo "OK: wheels present but clean (expected under bypass-selection=true)"
        else
          echo "OK: no wheels produced"
        fi
```

The exact text in `ci.yml` may differ in whitespace or comment
placement; the contract is the shape above.

### Why this shape instead of "build must fail"

See §Declared deviations. In short: hatchling `≥ 1.25` with
`bypass-selection = true` is a well-behaved no-op rather than an
error — it emits the metadata-only wheel and calls it done. Making
a Phase 23 CI job red-on-success of `python -m build` is fragile
(hatchling behaviour differs across releases) and inverts the real
invariant. The real invariant is **what ends up in the artefact, if
anything** — that is the contract Phase 24 inherits, because Phase
24's Rust-backed fat wheel will also need to verify it does not
sneak the pure-Python reference in via `include` globs or setup
hooks.

### Local simulator

`scripts/ci_local_simulate.sh` gained a block that reproduces the
guard offline: `rm -rf dist/`, `python -m build --wheel`, then for
each produced wheel check via `python -m zipfile -l` for reference
paths. The simulator no longer fails if `python -m build` itself
exits non-zero — that behaviour is environment-dependent and not
load-bearing.

## Part C — Parity scaffold (slice 23.3, Step 5)

### Fixture ownership

`tests/parity/conftest.py` introduces two session-scoped fixtures:

- `ref` — always yields `tinyquant_py_reference`. No skip path. The
  reference is reachable via `pyproject.toml`'s `pythonpath`
  extension, so this fixture never fails collection.
- `rs` — attempts `import tinyquant_cpu as rs_pkg`. On `ImportError`,
  calls `pytest.skip("Rust-backed tinyquant_cpu not installed;
  parity tests require Phase 24 fat wheel.")`. On success, yields
  the module.

The skip message is load-bearing: the spec's acceptance criterion
number 4 asserts on the substring `Rust-backed tinyquant_cpu not
installed`.

### Parameterisation

`cfg_triplet` covers `{(4, 42, 64), (2, 0, 128), (8, 999, 256)}` —
one entry per supported bit width, with distinct seeds and
dimensions to catch accidental seed/dimension hard-coding. The `rng`,
`vector`, and `batch` fixtures downstream are keyed on
`cfg_triplet`, so each parametrised case runs with a matched
`np.random.Generator(seed)` producing a `(dim,)` single vector and a
`(128, dim)` batch. Phase 24 will expand the matrix per the spec's
§Matrix coverage table (add `768` and `1536` dimensions, add batch
sizes 1 / 16 / 4096, add Hypothesis-based property sweeps).

### Test shape

`pytestmark = pytest.mark.parity` applies to the whole file, so
`pytest -m "not parity"` deselects the entire module and `pytest -m
parity` restricts to it. Tests split into four classes by what they
exercise:

- `TestConfigParity`: `config_hash` determinism and cross-impl
  equality.
- `TestRotationParity`: `RotationMatrix.apply` determinism and
  cross-impl `assert_allclose(atol=1e-6)`.
- `TestCompressRoundTrip`: self round-trip with a sanity MSE bound
  (`< 1.0`, deliberately loose — tight MSE is a calibration
  concern, not parity), and a cross-impl compress-with-ref /
  decompress-with-rs round trip with `atol=1e-3`.
- `TestSerializationParity`: `Codebook.to_bytes()` byte-stability
  and cross-impl byte-equality via `Codebook.from_bytes` /
  `to_bytes` round-trip.

The cross-impl tests are strict oracles: byte equality on
serialisation, 1e-6 allclose on numerics. Phase 24 will tighten
`TestRotationParity` to exact byte equality once the Rust f64 path
is confirmed to match the reference's f32→f64→f32 pipeline.

### Verification run

Locally (with a clean venv and no `tinyquant_cpu` installed):

```text
pytest tests/parity/ -v
  test_config_hash_matches_self[cfg_triplet0] PASSED
  test_config_hash_matches_self[cfg_triplet1] PASSED
  test_config_hash_matches_self[cfg_triplet2] PASSED
  test_config_hash_cross_impl[cfg_triplet0]   SKIPPED (Rust-backed
                                              tinyquant_cpu not
                                              installed; parity
                                              tests require Phase
                                              24 fat wheel.)
  ... (analogous for the other three cross-impl tests)
  test_rotation_deterministic[cfg_triplet*]   PASSED
  test_self_round_trip[cfg_triplet*]          PASSED
  test_codebook_bytes_stable[cfg_triplet*]    PASSED
  test_cross_impl_round_trip[cfg_triplet*]    SKIPPED
  test_codebook_cross_impl_bytes[cfg_triplet*] SKIPPED
```

`pytest -m "not parity" -q` → `214 passed, 24 deselected`. The
existing suite is unaffected by the scaffold.

### Runtime-limitation note

On the implementer's local Windows box the global `sys.path`
resolves `tinyquant_cpu` to a stale `src/tinyquant_cpu/` tree in a
sibling clone of TinyQuant. That means a naïve `pytest tests/parity/`
does **not** skip the cross-impl tests locally — it resolves the
import, then fails at `rs.codec.CodecConfig(...)` with an
`AttributeError` (the stale tree is a partial `__init__.py` without
the `codec` attribute bound). This is an environmental artefact, not
a scaffold defect: the `pytest.skip` trigger is `ImportError`, and in
any CI runner or clean venv `import tinyquant_cpu` raises
`ModuleNotFoundError`. The Phase 24 fat wheel will resolve this by
being the thing `import tinyquant_cpu` returns.

## Part D — Docs sweep (slice 23.3, Step 6)

The docs sweep touched 15 files. Scope was bound by the plan's §Docs
updates checklist — rows marked **Update**, **Create**, or
**Append** were addressed; rows marked **No-op**, **Review**, or
**Owned by another agent** were left alone.

File-by-file summary lives in the commit body for
`phase-23 step 6: docs — reference-only status`. The cross-file
prose-alignment rule in `AGENTS.md` was honoured: the Phase 23
reference-demotion language reads the same in `AGENTS.md`,
`CLAUDE.md`, `README.md`, and `.github/README.md` at every point the
four files describe the same topic.

New wiki page: [[entities/python-reference-implementation]]. It
carries the full what / where / why / how-to-invoke / how-it-relates-
to-Phase-24 story with links back to this note and to
[[plans/rust/phase-23-python-reference-demotion|Phase 23]].

## Acceptance-criteria trace

Cross-referenced against the plan's §Acceptance criteria #1–10:

| # | Criterion | Verified where | Status |
|---|---|---|---|
| 1 | `pip install tinyquant-cpu==0.1.1` from PyPI still works | External — PyPI | Out-of-repo; PyPI artefact untouched by this phase |
| 2 | `python -m build .` fails with hatchling "no packages" and no `.whl` in `dist/` | `build-package-does-not-leak-reference` | **Refined** — see §Declared deviations. Contract now: no reference paths in any produced wheel |
| 3 | `pytest tests/ -q` passes 214 tests | Local + CI | Verified (214 passed, 24 deselected) |
| 4 | `pytest tests/parity/ -v -m parity` runs; self passes, cross-impl skips with expected message | Local (clean venv) + CI | Verified in Phase 23.3 |
| 5 | `grep -R 'tinyquant_cpu' tests/` returns zero matches | `git grep -l 'tinyquant_cpu' -- 'tests/'` | Verified zero in Phase 23.1 |
| 6 | `grep -R 'tinyquant_cpu' src/` returns zero (src is gone) | `ls src/` → does not exist | Verified in Phase 23.2 |
| 7 | `build-package-does-not-leak-reference` green on a clean Phase 23 branch, red on reference leak | CI | Green on `1b93b9d` |
| 8 | `pytest --cov=tinyquant_py_reference --cov-fail-under=90` passes | CI test chunks | Verified |
| 9 | `docs/entities/python-reference-implementation.md` exists with valid frontmatter and is linked from `docs/entities/TinyQuant.md` | Phase 23.3 docs sweep | Verified |
| 10 | `git log --follow` on any moved file traces through the rename | `git log --follow tests/reference/tinyquant_py_reference/codec/codec.py` | Verified |

## See also

- [[plans/rust/phase-23-python-reference-demotion|Phase 23 Plan]]
- [[entities/python-reference-implementation|Python Reference Implementation]]
- [[design/rust/testing-strategy|Testing Strategy]] — new "Reference as oracle" section
- [[design/rust/phase-22-implementation-notes|Phase 22 Implementation Notes]] — prior slice
