---
title: "Phase 28.7: Canonical Rotation Mode — Cross-Impl Bit Parity"
tags:
  - plans
  - rust
  - phase-28.7
  - rotation
  - numerics
  - parity
date-created: 2026-04-23
status: planned
category: planning
---

# Phase 28.7: Canonical Rotation Mode — Cross-Impl Bit Parity

> [!info] Goal
> Wire the Python reference's `RotationMatrix` to the same ChaCha20 +
> Box-Muller + faer QR pipeline used by Rust, achieving bit-identical
> rotation matrices across implementations for any `(seed, dimension)`
> pair. Replace the legacy-mode statistical parity gate in
> `tests/parity/test_cross_impl.py::TestRotationParity::test_rotation_cross_impl`
> with tight numerical parity (`atol=1e-6`).
>
> [!note] Reference docs
>
> - [[design/rust/numerical-semantics|Numerical Semantics]] §Rotation — the
>   three-step determinism plan and canonical mode spec
> - [[design/rust/risks-and-mitigations|Risks and Mitigations]] R19 —
>   faer cross-platform QR nondeterminism at large `dim`
> - [[plans/rust/phase-13-rotation-numerics|Phase 13]] — ChaChaGaussianStream
>   origin
> - [[plans/rust/phase-24-python-fat-wheel-official|Phase 24]] — Python fat
>   wheel where canonical mode was deferred

## Background

Phase 13 introduced `ChaChaGaussianStream` (ChaCha20 + Box-Muller) as the
canonical Gaussian source for Rust's `RotationMatrix::build`. Phase 24
shipped the Rust-backed `tinyquant_cpu` fat wheel but left the Python
reference (`tinyquant_py_reference`) on its original NumPy PCG64 + LAPACK QR
path. The two RNG pipelines produce completely different rotation matrices for
the same seed.

As a result, `test_rotation_cross_impl` was rewritten in Phase 28.7
(pre-work, on branch `develop`) to check statistical parity (Pearson ρ ≥
0.9999 on pairwise cosine similarity preservation) rather than tight
numerical parity. That gate is correct and passes, but it is not the final
intended contract — the design specifies `canonical` mode produces bit-
identical matrices (`numerical-semantics.md` §Rotation, table row
`canonical`).

This phase delivers canonical mode.

## Prerequisites

- Phase 28.5 complete. ✅
- `tinyquant_cpu` 1.0.0 installed in the dev venv and `rs` fixture live. ✅
- `test_rotation_cross_impl` statistical gate green. ✅
- R19 (faer cross-platform nondeterminism) understood and scoped — see
  §Constraints below.

## Constraints

### C1. R19: faer QR nondeterminism at large `dim`

`faer::Mat::qr()` dispatches to SIMD kernels via `pulp` at runtime. At
`dim ≥ 384` on x86_64, AVX2 vs AVX-512 hosts produce different f64 bit
patterns even under `RAYON_NUM_THREADS=1`. At `dim = 64` the matrix falls
below all dispatch thresholds and is scalar-path only, giving reproducible
results.

**Resolution:** thread `faer::Parallelism::None` through
`RotationMatrix::build` and force a scalar (non-SIMD) QR path by passing the
parallelism hint through to `faer::linalg::qr::no_pivoting::compute`. This
makes the output deterministic across all host CPUs. Regenerate the `dim=768`
rotation fixture from the scalar kernel and re-enable the bit-exact test
that Phase 14 `#[ignore]`d.

### C2. Python canonical bridge options

`numerical-semantics.md` §Rotation Step 3 lists two options for the bridge:

| Option | Description | When to use |
|--------|-------------|-------------|
| **PyO3 bridge** | `tinyquant_cpu._core` exposes `_chacha_gaussian_matrix(seed, dim)` returning a `numpy.ndarray` of the row-major f64 Gaussian matrix before QR; Python then calls `np.linalg.qr` + sign correction | Requires Rust extension installed; gives exact byte match on Gaussian fill but still uses LAPACK QR |
| **Full delegation** | Python calls `tinyquant_cpu._core.codec.RotationMatrix.from_config(cfg).matrix_f64_bytes()` and wraps the result | Exact byte match on the final matrix; delegates QR entirely to Rust |

**Full delegation is correct and simpler.** Option A (PyO3 Gaussian bridge)
still diverges at the QR step (LAPACK vs faer). Use Option B: expose
`RotationMatrix.matrix_f64_bytes()` from the PyO3 binding and have the
Python reference reconstruct the matrix from those bytes when `tinyquant_cpu`
is available.

### C3. Frozen oracle constraint

`tests/reference/tinyquant_py_reference/` is the frozen differential oracle.
Its `RotationMatrix` may be updated only when a documented rollout plan
explicitly requires it — this plan is that document. The change is
additive: `_cached_build` gains an override path, the default (NumPy PCG64)
is preserved as the legacy fallback.

## Deliverables

### D1. `RotationMatrix.matrix_f64_bytes()` PyO3 binding

Add a method to the `RotationMatrix` PyO3 class in
`rust/crates/tinyquant-py/src/codec.rs` that returns the raw row-major f64
bytes of the matrix as a Python `bytes` object.

```rust
fn matrix_f64_bytes(&self) -> Vec<u8> {
    self.inner
        .matrix()
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}
```

Expose in `tinyquant_cpu._core.codec.RotationMatrix`.

### D2. `RotationMatrix.faer_parallelism_none` fix (R19)

In `rust/crates/tinyquant-core/src/codec/rotation_matrix.rs`, replace the
`a.qr()` call with the explicit scalar-path API:

```rust
use faer::linalg::qr::no_pivoting;
use faer::Parallelism;

let qr = no_pivoting::compute::qr_in_place(
    a.as_mut(),
    householder_coeffs.as_mut(),
    Parallelism::None,
    stack,
    Default::default(),
);
```

Regenerate `tests/fixtures/rotation/seed_42_dim_768.f64.bin` from the scalar
kernel and re-enable the `#[ignore]`d bit-exact fixture test in
`rust/crates/tinyquant-core/tests/rotation_matrix.rs`.

### D3. `_install_canonical_rotation` in Python reference

Add a module-level function to
`tests/reference/tinyquant_py_reference/codec/rotation_matrix.py`:

```python
def _install_canonical_rotation() -> None:
    """Override RotationMatrix._cached_build to use the Rust canonical path.

    Requires tinyquant_cpu to be installed. After calling this, every
    RotationMatrix.from_config call produces a matrix byte-identical to
    the Rust implementation's output for the same (seed, dimension).

    Call once at import time from the parity test conftest when the rs
    fixture is live.
    """
    import tinyquant_cpu._core as _core  # noqa: PLC0415

    @staticmethod  # type: ignore[misc]
    @functools.lru_cache(maxsize=8)
    def _canonical_build(seed: int, dimension: int) -> "RotationMatrix":
        rs_rot = _core.codec.RotationMatrix.from_config(
            _core.codec.CodecConfig(bit_width=4, seed=seed, dimension=dimension)
        )
        raw = rs_rot.matrix_f64_bytes()
        mat = np.frombuffer(raw, dtype="<f8").reshape(dimension, dimension).copy()
        mat.flags.writeable = False
        return RotationMatrix(matrix=mat, seed=seed, dimension=dimension)

    RotationMatrix._cached_build = _canonical_build  # type: ignore[method-assign]
```

### D4. Parity conftest: activate canonical mode when `rs` is live

In `tests/parity/conftest.py`, add a session-scoped autouse fixture that
installs the canonical rotation override when `tinyquant_cpu` is importable:

```python
@pytest.fixture(scope="session", autouse=True)
def _canonical_rotation_mode() -> None:
    """Activate canonical rotation when the Rust extension is available."""
    try:
        from tinyquant_py_reference.codec.rotation_matrix import (
            _install_canonical_rotation,
        )
        _install_canonical_rotation()
    except (ImportError, AttributeError):
        pass
```

### D5. Upgrade rotation and round-trip parity tests to canonical gates

Once D1–D4 land, revert the two legacy-mode gates introduced in the
Phase 28.7 pre-work:

**`test_rotation_cross_impl`** — restore tight numerical parity:

```python
def test_rotation_cross_impl(
    self,
    ref: ModuleType,
    rs: ModuleType,
    cfg_triplet: tuple[int, int, int],
    vector: npt.NDArray[np.float32],
) -> None:
    """Canonical mode: Python and Rust rotation agree within 1e-6 on the same vector."""
    bw, seed, dim = cfg_triplet
    py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
    rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
    py_rot = ref.codec.RotationMatrix.from_config(py_cfg)
    rs_rot = rs.codec.RotationMatrix.from_config(rs_cfg)
    np.testing.assert_allclose(
        py_rot.apply(vector), rs_rot.apply(vector), atol=1e-6
    )
```

**`test_cross_impl_round_trip`** — restore full cross-impl decompression parity:

```python
for row in batch[:8]:
    py_cv = py_codec.compress(row, py_cfg, py_cb)
    rs_cv = rs.codec.CompressedVector.from_bytes(py_cv.to_bytes())
    np.testing.assert_allclose(
        py_codec.decompress(py_cv, py_cfg, py_cb),
        rs_codec.decompress(rs_cv, rs_cfg, rs_cb),
        atol=1e-3,
    )
```

## Files changed

| File | Change |
|------|--------|
| `rust/crates/tinyquant-py/src/codec.rs` | Add `matrix_f64_bytes` PyO3 method to `RotationMatrix` |
| `rust/crates/tinyquant-core/src/codec/rotation_matrix.rs` | Pass `Parallelism::None` to faer QR (R19 fix) |
| `rust/crates/tinyquant-core/tests/rotation_matrix.rs` | Re-enable `#[ignore]`d bit-exact fixture test |
| `rust/crates/tinyquant-core/tests/fixtures/rotation/seed_42_dim_768.f64.bin` | Regenerate from scalar kernel |
| `tests/reference/tinyquant_py_reference/codec/rotation_matrix.py` | Add `_install_canonical_rotation()` |
| `tests/parity/conftest.py` | Add `_canonical_rotation_mode` autouse session fixture |
| `tests/parity/test_cross_impl.py` | Revert statistical rotation gate → tight numerical parity; revert byte-only cross-impl gate → full decompression parity |

## Test plan

1. `cargo test -p tinyquant-core rotation_matrix` — bit-exact fixture test
   re-enabled and green on at least two CI runners with different CPUs.
2. `pytest tests/parity/ -v -k rotation` — all three `cfg_triplet`
   parametrizations pass with `atol=1e-6`.
3. `pytest tests/parity/ -v` — full parity suite green (no regressions from
   D3/D4 changes to the Python reference).
4. `cargo test --workspace` — no regressions from R19 scalar-path fix.

## Risks

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| faer scalar-path API differs across minor versions | Low | Pin `faer` to a specific minor version in `Cargo.lock`; update fixture regeneration recipe in this doc if faer is upgraded |
| `_install_canonical_rotation` leaks state between test sessions | Low | Uses `lru_cache` which is session-scoped; conftest fixture is `scope="session"` |
| `CodecConfig` seed type mismatch (Python `int` vs Rust `u64`) | Low | `_canonical_build` receives `seed: int` from Python's `lru_cache` key; Rust `CodecConfig` accepts `seed: u64` via PyO3 type coercion — verify with seed=0 edge case |
| dim=768 scalar-path fixture disagrees with previous `#[ignore]`d value | Certain (R19 was the cause) | Regenerate fixture; document regeneration command in this file |

## Regenerating the dim=768 fixture

After the R19 scalar-path fix lands:

```bash
cd rust
cargo xtask fixtures refresh-rotation --seed 42 --dim 768
```

If `xtask` does not yet support `refresh-rotation`, run:

```bash
cargo test -p tinyquant-core -- rotation_matrix::dump_fixture --ignored --nocapture
```

and copy the printed bytes to
`tests/fixtures/rotation/seed_42_dim_768.f64.bin`.
