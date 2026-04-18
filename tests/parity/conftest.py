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
