"""Smoke tests verifying the TinyQuant package is importable."""


def test_import_tinyquant() -> None:
    """Verify that tinyquant can be imported and exposes a version string."""
    import tinyquant

    assert tinyquant.__version__ == "0.1.0"
