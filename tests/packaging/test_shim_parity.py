"""Name-parity audit: fat-wheel shim vs. Phase 23 pure-Python reference.

The fat wheel's `tinyquant_cpu.{codec,corpus,backend}` sub-packages must
re-export the exact same public names as the Phase 23 reference package
`tinyquant_py_reference.{codec,corpus,backend}`. This test is the
load-bearing guard against accidental drift when symbols are added to
one side but not the other.

Skipped cleanly when `tinyquant_rs` is not installed — the dev-mode
shim lives in `src/tinyquant_cpu/` but imports from `tinyquant_rs._core`,
so the Phase 22.A extension must be on the path for this audit to run.
See `docs/plans/rust/phase-24-python-fat-wheel-official.md`
§Name-parity audit for the contract.
"""

from __future__ import annotations

import importlib

import pytest

# The dev-mode shim re-exports via `tinyquant_rs._core`. Without the
# Phase 22.A extension installed, `import tinyquant_cpu` itself fails,
# so we cannot assert anything about its name table. Skip the module
# as a whole rather than erroring out collection.
pytest.importorskip(
    "tinyquant_rs",
    reason=(
        "tinyquant_rs (Phase 22.A PyO3 extension) is not installed; "
        "shim-parity audit requires the Rust backend to load."
    ),
)

SUBPKGS = ["codec", "corpus", "backend"]


@pytest.mark.parametrize("subpkg", SUBPKGS)
def test_shim_exports_match_reference(subpkg: str) -> None:
    """Assert shim and reference share the same public-name set."""
    shim = importlib.import_module(f"tinyquant_cpu.{subpkg}")
    ref = importlib.import_module(f"tinyquant_py_reference.{subpkg}")
    shim_names = set(shim.__all__)
    ref_names = set(ref.__all__)
    missing = ref_names - shim_names
    extra = shim_names - ref_names
    assert not missing, f"{subpkg}: shim missing names {missing}"
    assert not extra, f"{subpkg}: shim has extra names {extra}"
