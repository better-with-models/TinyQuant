---
title: "Phase 24 Implementation Notes"
tags:
  - implementation-notes
  - rust
  - phase-24
  - python
  - distribution
  - fat-wheel
  - packaging
date-created: 2026-04-14
status: complete
category: design
---

# Phase 24 Implementation Notes

## Summary

Phase 24 ships every in-repo deliverable required to publish a
Rust-backed `tinyquant-cpu==0.2.0` fat wheel that reclaims the PyPI
slot vacated by the Phase 23 reference demotion. The fat wheel
preserves the `import tinyquant_cpu` surface byte-for-byte so
downstream code requires no source-level changes. Phase 24 landed in
four sub-slices on `phase-22-25-release-chain`:

- **Phase 24.1 — Dev shim + shim templates (LANDED at `12a7191`,
  red at `6090f06`).** Added the dev-mode `src/tinyquant_cpu/`
  shim that re-exports `tinyquant_rs._core` under the `tinyquant_cpu`
  import name so editors and the cross-impl parity suite can resolve
  the new name immediately. The same files are templated under
  `scripts/packaging/templates/` and become the package tree of the
  fat wheel verbatim.
- **Phase 24.2 — Fat-wheel assembler (LANDED at `adf4008`, red at
  `4938534`, follow-up `a1b1af4`).** Added
  `scripts/packaging/assemble_fat_wheel.py` plus `tests/packaging/
  test_assemble_fat_wheel.py`. The assembler is pure stdlib
  (`zipfile` + `hashlib`), emits PEP 376–valid `RECORD` (url-safe
  base64, empty hash/size for `RECORD` itself), uses a fixed
  `_DETERMINISTIC_MTIME` for byte-reproducible output, and validates
  every per-arch input wheel against `REQUIRED_PLATFORM_KEYS`.
- **Phase 24.3 — CI workflow + publish-guard contract (LANDED at
  `af641a9`, `7033224`, `72cf7f1`, quality fix `1c4e844`).** Added
  `.github/workflows/python-fatwheel.yml` shaped after Phase 22.D's
  `rust-release.yml`: `release-gate` job, `dry_run` workflow_dispatch
  input defaulting `true`, byte-identical `publish.if` guard enforced
  by `cargo xtask check-publish-guards` against
  `rust/xtask/src/cmd/guard_sync_python.rs::CONTRACT`, dry-run
  fabrication of dummy per-arch wheels via the extracted
  `scripts/packaging/_make_dummy_wheel.py`, and a `lint-workflow`
  guard with an `npx`-absent no-op fallback.
- **Phase 24.4 — Parity audit + docs (this slice).** No code change.
  Confirms the `tests/parity/conftest.py::rs` fixture shipped in
  Phase 23.3 already imports `tinyquant_cpu` and that Phase 24.1's
  in-tree shim makes the import live in dev environments without
  installing any wheel. Records the AC trace, the six declared
  deviations carried forward from Phase 24.3, and the test
  inventory.

## Declared deviations

The Phase 24.3 plan callout (lines 832-872 of
`docs/plans/rust/phase-24-python-fat-wheel-official.md`) named six
intentional departures of the shipped workflow from the literal YAML
in §CI workflow. They are reproduced here so the design note is
self-contained.

### 1. Added `release-gate` job + `dry_run` workflow_dispatch input

> Mirrors the Phase 22.D pattern from `rust-release.yml`. The shell
> regex is `^py-v[0-9]+\.[0-9]+\.[0-9]+$` (only the prefix differs
> from `rust-v`); workflow_dispatch always sets
> `should_publish=false`. The `dry_run` input defaults to `true` so
> manual rehearsals never publish.

Evidence: `.github/workflows/python-fatwheel.yml:49,76,80,94,97,99`.

### 2. `publish.if` guard contract

> Re-uses the Phase 22.D contract expression byte-for-byte:
> `needs.release-gate.outputs.should_publish == 'true' &&
> inputs.dry_run != true`. Enforced in CI by
> `cargo xtask check-publish-guards` against
> `rust/xtask/src/cmd/guard_sync_python.rs::CONTRACT`.

Evidence: `.github/workflows/python-fatwheel.yml:336-337`;
`rust/xtask/src/cmd/guard_sync_python.rs:35` (`const CONTRACT`),
`:68` (`fn check`).

### 3. `fetch-inputs` dry-run fabrication

> When the upstream `rust-v<version>` release does not exist (manual
> dispatch against `main`), the job fabricates five minimal dummy
> per-arch wheels with an inline Python script so downstream jobs
> still exercise end-to-end. Tag-push runs continue to use
> `gh release download` against the real release.

The Phase 24.3 quality-fix commit (`1c4e844`) extracted the inline
script into `scripts/packaging/_make_dummy_wheel.py` so the
workflow only invokes it. Evidence:
`.github/workflows/python-fatwheel.yml` — `fetch-inputs` job;
`scripts/packaging/_make_dummy_wheel.py`.

### 4. `SOURCE_DATE_EPOCH` resolution in `assemble`

> Derived from the tagged commit's committer-timestamp on tag-push
> runs, fixed to `1776038400` (2026-04-13T00:00:00Z, matching the
> assembler's fixed mtime) on dry-run. Ensures the produced fat
> wheel is byte-reproducible across re-runs.

The fixed mtime in the assembler is
`_DETERMINISTIC_MTIME = (2026, 4, 13, 0, 0, 0)` —
`scripts/packaging/assemble_fat_wheel.py:61-63`.

### 5. Belt-and-braces dry-run guard on the publish step

> The `pypa/gh-action-pypi-publish` step carries
> `if: inputs.dry_run != true` in addition to the job-level guard so
> a misconfigured downstream edit still cannot reach PyPI.

Evidence: `.github/workflows/python-fatwheel.yml:363`.

### 6. `lint-workflow` job

> Wraps `npx --yes @action-validator/cli` against this workflow and
> short-circuits to a no-op if `npx` is absent (Linux GitHub-hosted
> runners always ship Node).

Evidence: `.github/workflows/python-fatwheel.yml:132` —
`needs: [release-gate, lint-workflow]` on the next-stage job; the
`lint-workflow` job is defined earlier in the same file.

## Phase 24.4 parity-fixture note

This is not a declared deviation — it is a confirmation that a spec
item shipped out-of-order. The plan's §Prerequisites bullet
("`tests/parity/test_cross_impl.py` […] `rs` fixture that currently
points at `tinyquant_rs`; this phase flips the fixture to the fat
wheel once assembled") was satisfied out-of-order. The fixture in
`tests/parity/conftest.py:34` already reads
`import tinyquant_cpu as rs_pkg`. In Phase 24.1 the in-tree
`src/tinyquant_cpu/__init__.py` was made into a shim that re-exports
`tinyquant_rs._core` under `sys.modules["tinyquant_cpu._core"]`, so
the existing import path is **already** the load path the fat wheel
will install. Phase 24.4 verifies this and does not edit the
fixture.

## Architecture summary

### Dev shim (`src/tinyquant_cpu/__init__.py`)

In-tree at `src/tinyquant_cpu/__init__.py:1-29`. Two responsibilities:

1. `import tinyquant_rs._core as _core` — pulls in the locally
   installed Rust extension.
2. `sys.modules["tinyquant_cpu._core"] = _core` — re-homes it so the
   sub-package shims (`codec`, `corpus`, `backend`) can resolve
   attributes off `sys.modules["tinyquant_cpu._core"]` exactly the
   way they will inside the fat wheel.

The dev shim deliberately **skips** the platform selector: in dev,
the Rust extension is already locked to the host arch. The same
sub-package shims (`scripts/packaging/templates/codec__init__.py`
etc.) are reused unchanged — the only file that differs between dev
and fat wheel is `__init__.py` (the fat-wheel version runs
`_selector.load_core()` first).

### Fat-wheel assembler (`scripts/packaging/assemble_fat_wheel.py`)

Pure stdlib repackager — no rebuild happens. The flow is:

1. `discover_inputs()` (lines 122-165): glob `tinyquant_rs-*.whl`
   under `--input-dir`, parse with `WHEEL_NAME_RE`, validate
   `--version`, map every per-arch tag to a canonical
   `_lib/<key>/` directory name via `PLATFORM_KEY_BY_TAG`, assert
   `REQUIRED_PLATFORM_KEYS` are all present.
2. `build_fat_wheel()` (lines 203-270): emit a single
   `py3-none-any.whl` zip using `_deterministic_zipinfo()` for
   byte-reproducibility. Order is fixed: shim Python sources first,
   then per-arch `_core` extensions sorted by platform key, then
   `dist-info/{METADATA, WHEEL, LICENSE}`, and finally `RECORD` —
   whose own line in the file uses empty hash and size fields per
   PEP 376.
3. Output: the `.whl` plus a sibling `.manifest.json` recording
   per-input shas, the assembled wheel sha, sizes, assembler
   version, and build timestamp (lines 316-335).

The assembler is **structurally** unable to leak `tests/reference/`
paths: `build_fat_wheel()` only writes the explicit arcnames
`tinyquant_cpu/...`, `tinyquant_cpu/_lib/...`, and
`tinyquant_cpu-<ver>.dist-info/...` (lines 233-265). There is no
glob of the source tree.

### Runtime selector (`scripts/packaging/templates/_selector.py`)

Templated into the fat wheel verbatim. Detects
`(sys.platform, platform.machine(), libc)`, normalises arch via the
`_ARCH_ALIASES` table (handles Windows `AMD64` vs Linux `aarch64`
vs macOS `arm64`), maps to `_lib/<key>/_core.<ext>`, verifies the
binary's magic bytes (`\x7fELF`, Mach-O families, or `MZ`), and
loads it via `importlib.util.spec_from_file_location`. Failure paths
raise `UnsupportedPlatformError` (subclass of `ImportError`) with the
detected tuple and a source-build URL — the §Risks "loud fail"
principle.

### Byte-identical CONTRACT guard

`rust/xtask/src/cmd/guard_sync_python.rs:35` defines a
`const CONTRACT: &str` whose body is the exact `if:` expression
shipped in `python-fatwheel.yml:336-337`. The `check()` function
(`:68`) walks the YAML, locates the `publish.if` block, and asserts
byte-equality. Mismatch fails CI before any wheel is built. A unit
test under the same file (`MATCHES_CONTRACT` at line 191, asserted
at line 252) keeps the contract self-tested.

## Acceptance-criteria trace

Cross-referenced against the plan's §Acceptance criteria #1–12.
Status legend: ✅ live in this branch, ⏳ live-on-CI (requires the
fat-wheel install path to execute), ⚠️ deferred until tag push.

| # | Criterion | Verified where | Status |
|---|---|---|---|
| 1 | `pip install tinyquant-cpu==0.2.0` works on all 5 Tier-1 runners | `python-fatwheel.yml::install-test` matrix | ⏳ live-on-CI; first real-tag run will exercise |
| 2 | `detect_platform_key()` returns the expected key on every Tier-1 runner | `python-fatwheel.yml::install-test` "Smoke import + key detection" step | ⏳ live-on-CI |
| 3 | Unsupported host raises `UnsupportedPlatformError` naming the detected tuple | `tests/packaging/test_selector_detection.py::test_detect_platform_key_unsupported_machine` (line 162 in collected list) | ✅ live |
| 4 | `tests/parity/test_cross_impl.py` passes with **zero** diffs against the reference on the parametrised matrix | `tests/parity/test_cross_impl.py` (24 collected, 12 cross-impl) | ⏳ live in dev (shim resolves `tinyquant_cpu`); live-on-CI for the full Tier-1 matrix |
| 5 | `tests/packaging/test_shim_parity.py` asserts shim `__all__` equals reference `__all__` per sub-package | `tests/packaging/test_shim_parity.py` (3 parametrised cases over `codec, corpus, backend`) | ✅ live |
| 6 | Fat wheel is `< 50 MB` | `python-fatwheel.yml::assemble` "Size gate (< 50 MB)" step | ⏳ live-on-CI |
| 7 | Fat wheel contains exactly five `_core` binaries | `python-fatwheel.yml::assemble` "Binary-count gate (exactly 5)" step | ⏳ live-on-CI; also enforced structurally by `discover_inputs()` checking `REQUIRED_PLATFORM_KEYS` (`scripts/packaging/assemble_fat_wheel.py:91-97,159-164`) |
| 8 | `twine check dist/tinyquant_cpu-*.whl` passes | `tests/packaging/test_assemble_fat_wheel.py::test_twine_check_passes` (line 435) | ✅ live |
| 9 | Fat wheel `RECORD` sha256 entries validate; `wheel unpack` / `wheel pack` round-trip is byte-identical | `tests/packaging/test_python_fatwheel_workflow.py::test_wheel_unpack_pack_round_trip`; `tests/packaging/test_assemble_fat_wheel.py::test_wheel_roundtrip_self_validation` (line 371) | ✅ live |
| 10 | Fat wheel version (parsed from filename) is strictly greater than `0.1.1` | `python-fatwheel.yml::publish` "Verify version > 0.1.1" step; `tests/packaging/test_python_fatwheel_workflow.py::test_version_gate_*` | ✅ live (workflow dry-run harness exercises the gate) |
| 11 | No file under `tests/reference/` appears in the fat wheel; Phase 23 `build-package-does-not-leak-reference` passes | `scripts/packaging/assemble_fat_wheel.py:233-265` (the assembler only ever writes `tinyquant_cpu/...` and `<dist-info>/...` arcnames; no glob of the source tree) plus the inherited Phase 23 wheel-leak CI guard | ✅ live (structurally guaranteed by the assembler and re-verified by the Phase 23 guard) |
| 12 | sdist contains a `pyproject.toml` with a `[tool.maturin]` fallback block so `pip install --no-binary tinyquant-cpu tinyquant-cpu` works on unsupported platforms | — | ⚠️ deferred to Phase 25/26: this branch ships the binary fat wheel only; the maturin source-build fallback is out of scope for the four landed sub-slices |

## Test inventory

Phase 24 tests live under two roots: `tests/parity/` (cross-impl
parity, scaffolded in Phase 23.3, populated in Phase 24.4 by virtue
of the dev shim) and `tests/packaging/` (the four files added by
Phases 24.1–24.3).

| File | Tests | Asserts |
|---|---:|---|
| `tests/parity/test_cross_impl.py` | 24 (12 self, 12 cross-impl) | `config_hash` self + cross-impl equality; `RotationMatrix.apply` determinism + `assert_allclose(atol=1e-6)`; self compress round-trip with MSE bound + cross-impl round-trip with `atol=1e-3`; `Codebook.to_bytes()` byte-stability and cross-impl byte-equality. The cross-impl half resolves through the in-tree shim in dev and through the fat wheel on CI. |
| `tests/packaging/test_shim_parity.py` | 3 | For each of `codec`, `corpus`, `backend`: `set(shim.__all__) == set(ref.__all__)` — the load-bearing name-drift guard. |
| `tests/packaging/test_selector_detection.py` | 16 | Per-platform `detect_platform_key()` truth table for the six supported tuples; rejection of unsupported machines / musl-aarch64; SOABI-fallback libc detection; `_ext_filename` shape; selector public surface. |
| `tests/packaging/test_assemble_fat_wheel.py` | 10 | `_sha256` PEP 376 encoding; `discover_inputs` happy-path / version-mismatch / missing-platform; `extract_core_extension` for Linux+Windows; full end-to-end `build_fat_wheel` smoke; `RECORD` self-validation round-trip; `twine check` passes. |
| `tests/packaging/test_python_fatwheel_workflow.py` | 11 | Workflow YAML invariants: every job has `runs-on`; no orphan `needs:`; `release-gate` is a `publish` dependency; `publish.if` byte-equals the contract; `dry_run` defaults `true`; concurrency cancel-in-progress is `false`; `release-gate` regex is `^py-v…$`; assembler produces a valid wheel; unpack/pack round-trip is byte-identical; version gate rejects dry-run version and accepts a real release version. |

Total Phase 24 surface: **64 collected** (24 parity + 40 packaging),
of which 12 are skipped in the absence of the live fat wheel
(cross-impl parity tests when the dev shim is unavailable). On this
worktree the full suite ran clean (262 passed, 13 skipped) including
all Phase 24 surface.

## Slice provenance

Commits per sub-slice on `phase-22-25-release-chain`:

| Slice | Commit(s) | Subject |
|---|---|---|
| 24.1 (red) | `6090f06` | `test(phase-24.1): add shim-parity and selector-detection tests (red)` |
| 24.1 (green) | `12a7191` | `feat(phase-24.1): add fat-wheel shim templates and dev-mode package` |
| 24.2 (red) | `4938534` | `test(phase-24.2): add fat-wheel assembler unit tests (red)` |
| 24.2 (green) | `adf4008` | `feat(phase-24.2): add fat-wheel assembler script with PEP 376 RECORD` |
| 24.2.5 | `a1b1af4` | `fix(phase-24.2.5): bump METADATA template to PEP 639 v2.4` |
| 24.3 (workflow) | `af641a9` | `feat(phase-24.3): add python-fatwheel workflow with release-gate` |
| 24.3 (xtask guard) | `7033224` | `feat(phase-24.3): extend xtask guard-sync for python-fatwheel publish if` |
| 24.3 (test) | `72cf7f1` | `test(phase-24.3): add workflow dry-run harness` |
| 24.3 (quality fix) | `1c4e844` | `fix(phase-24.3): address quality review — extract dummy-wheel script, tighten gating` |
| 24.4 (this slice) | — | docs only; AC trace + plan flip + cross-file alignment |

The cross-file prose alignment touched in the merge commit `3abaa55`
("`docs+test: align cross-file prose and parity scaffold with merged
baseline`") landed the AGENTS.md / CLAUDE.md sentences naming the
Phase 24.1 dev shim. README.md and `.github/README.md` already
mention `0.2.0` and the fat wheel from earlier passes; Phase 24.4
verified them and made no edits there.

## See also

- [[plans/rust/phase-24-python-fat-wheel-official|Phase 24 Plan]]
- [[plans/rust/phase-23-python-reference-demotion|Phase 23 Plan]]
- [[design/rust/phase-23-implementation-notes|Phase 23 Implementation Notes]] — prior slice
- [[design/rust/phase-22-implementation-notes|Phase 22 Implementation Notes]] — upstream Rust release
- [[research/python-rust-wrapper-distribution|Distribution Research §Approach B]] — fat-wheel rationale
