"""Smoke tests verifying the TinyQuant package is importable."""

import re


def test_import_tinyquant() -> None:
    """Verify that tinyquant_cpu can be imported and exposes a version string."""
    import tinyquant_cpu

    assert isinstance(tinyquant_cpu.__version__, str)
    assert re.match(r"^\d+\.\d+\.\d+", tinyquant_cpu.__version__), (
        f"__version__ must start with a semver triplet, got: "
        f"{tinyquant_cpu.__version__!r}"
    )
