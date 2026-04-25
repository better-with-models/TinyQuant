"""Cross-implementation parity fixtures.

Phase 23: only the pure-Python reference is wired in. The ``rs``
fixture raises ``pytest.skip`` until Phase 24 installs the Rust-backed
``tinyquant_cpu`` fat wheel into the venv.

Phase 24 will replace the skip with::

    import tinyquant_cpu as rs   # the Rust-backed fat wheel

and every parity test becomes a live assertion.

Phase 28.7: the ``_canonical_rotation_mode`` autouse fixture activates
canonical rotation mode when ``tinyquant_cpu`` is installed, overriding
the Python reference's ``RotationMatrix._cached_build`` to use the same
ChaCha20 + faer QR pipeline as Rust. This enables bit-identical rotation
matrices across implementations.
"""

from __future__ import annotations

from collections.abc import Iterator
from typing import Any, cast

import numpy as np
import numpy.typing as npt
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
        import tinyquant_cpu as rs_pkg
        import tinyquant_cpu.backend  # noqa: F401
        import tinyquant_cpu.codec  # noqa: F401
        import tinyquant_cpu.corpus  # noqa: F401
    except ImportError:
        pytest.skip(
            "Rust-backed tinyquant_cpu not installed; "
            "parity tests require Phase 24 fat wheel."
        )
    else:
        yield rs_pkg


@pytest.fixture(scope="session", autouse=True)
def _canonical_rotation_mode() -> None:
    """Activate canonical rotation when the Rust extension is available.

    When ``tinyquant_cpu`` is installed, overrides
    ``RotationMatrix._cached_build`` in the Python reference so that
    every ``RotationMatrix.from_config`` call produces a matrix
    byte-identical to the Rust implementation for the same
    ``(seed, dimension)`` pair. Falls back silently when the package is
    absent so legacy-mode tests are unaffected.
    """
    try:
        import tinyquant_cpu  # noqa: F401, PLC0415

        from tinyquant_py_reference.codec.rotation_matrix import (
            RotationMatrix,
            _install_canonical_rotation,
        )

        _install_canonical_rotation()
        # Loud-failure guard: a future regression that silently leaves
        # _cached_build pointing at the legacy NumPy QR path would let
        # parity tests pass against the wrong baseline. The canonical
        # closure is always named "_canonical_build".
        assert RotationMatrix._cached_build.__wrapped__.__name__ == (
            "_canonical_build"
        ), (
            "_install_canonical_rotation did not patch _cached_build; "
            "parity tests would silently run in legacy mode"
        )
    except ImportError:
        pass


@pytest.fixture(
    params=[(4, 42, 64), (2, 0, 128), (8, 999, 256), (4, 42, 512), (4, 42, 768)],
)
def cfg_triplet(request: pytest.FixtureRequest) -> tuple[int, int, int]:
    """(bit_width, seed, dimension) tuples covering every bit-width.

    Dim ≥ 384 cases (512, 768) exceed the faer parallel-kernel dispatch
    threshold and provide cross-impl rotation-parity coverage at
    production-relevant dimensions. The dim=768 case matches the
    production fixture point (ROTATION_GOLD_SET) and the bit-exact Rust
    test `cargo test -p tinyquant-core rotation_fixture_parity` (see
    rotation_fixture_parity.rs::seed_42_dim_768_matches_frozen_snapshot_bit_for_bit).
    """
    return cast(tuple[int, int, int], request.param)


@pytest.fixture()
def rng(cfg_triplet: tuple[int, int, int]) -> np.random.Generator:
    """Seeded NumPy Generator derived from the ``cfg_triplet`` seed."""
    _, seed, _ = cfg_triplet
    return np.random.default_rng(seed)


@pytest.fixture()
def vector(
    rng: np.random.Generator, cfg_triplet: tuple[int, int, int]
) -> npt.NDArray[np.float32]:
    """Single FP32 vector of the dimension specified by ``cfg_triplet``."""
    _, _, dim = cfg_triplet
    return rng.standard_normal(dim).astype(np.float32)


@pytest.fixture()
def batch(
    rng: np.random.Generator, cfg_triplet: tuple[int, int, int]
) -> npt.NDArray[np.float32]:
    """FP32 matrix of 128 vectors with the dimension specified by ``cfg_triplet``."""
    _, _, dim = cfg_triplet
    return rng.standard_normal((128, dim)).astype(np.float32)
