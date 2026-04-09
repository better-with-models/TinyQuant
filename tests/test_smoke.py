"""Smoke tests verifying the TinyQuant package is importable."""


def test_import_tinyquant() -> None:
    """Verify that tinyquant_cpu can be imported and exposes a version string."""
    import tinyquant_cpu

    assert tinyquant_cpu.__version__ == "0.1.0"
