"""Smoke tests verifying the TinyQuant package is importable."""

import re


def test_import_tinyquant() -> None:
    """Verify that tinyquant_py_reference can be imported and exposes a version string."""
    import tinyquant_py_reference

    assert isinstance(tinyquant_py_reference.__version__, str)
    assert re.match(r"^\d+\.\d+\.\d+", tinyquant_py_reference.__version__), (
        f"__version__ must start with a semver triplet, got: "
        f"{tinyquant_py_reference.__version__!r}"
    )
