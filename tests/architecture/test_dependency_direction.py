"""Architecture tests: dependency direction and export consistency (V-01)."""

from __future__ import annotations

import contextlib
import importlib
import sys
from collections.abc import Generator
from typing import Any

import pytest


@contextlib.contextmanager
def _clean_tinyquant_imports() -> Generator[None, None, None]:
    """Remove tinyquant_py_reference modules from sys.modules, then restore on exit."""
    saved: dict[str, Any] = {
        k: v for k, v in sys.modules.items() if k.startswith("tinyquant_py_reference")
    }
    for key in list(sys.modules):
        if key.startswith("tinyquant_py_reference"):
            del sys.modules[key]
    try:
        yield
    finally:
        # Remove anything the test imported
        for key in list(sys.modules):
            if key.startswith("tinyquant_py_reference"):
                del sys.modules[key]
        # Restore originals
        sys.modules.update(saved)


class TestDependencyDirection:
    """Codec must not depend on corpus or backend; corpus must not depend on backend."""

    def test_codec_does_not_import_corpus(self) -> None:
        """Importing tinyquant_py_reference.codec brings no corpus modules."""
        with _clean_tinyquant_imports():
            importlib.import_module("tinyquant_py_reference.codec")
            corpus_modules = [
                k for k in sys.modules if k.startswith("tinyquant_py_reference.corpus")
            ]
            assert corpus_modules == [], (
                f"codec imported corpus modules: {corpus_modules}"
            )

    def test_codec_does_not_import_backend(self) -> None:
        """Importing tinyquant_py_reference.codec brings no backend modules."""
        with _clean_tinyquant_imports():
            importlib.import_module("tinyquant_py_reference.codec")
            backend_modules = [
                k for k in sys.modules if k.startswith("tinyquant_py_reference.backend")
            ]
            assert backend_modules == [], (
                f"codec imported backend modules: {backend_modules}"
            )

    def test_corpus_does_not_import_backend(self) -> None:
        """Importing tinyquant_py_reference.corpus brings no backend modules."""
        with _clean_tinyquant_imports():
            importlib.import_module("tinyquant_py_reference.corpus")
            backend_modules = [
                k for k in sys.modules if k.startswith("tinyquant_py_reference.backend")
            ]
            assert backend_modules == [], (
                f"corpus imported backend modules: {backend_modules}"
            )


_PACKAGES = (
    "tinyquant_py_reference.codec",
    "tinyquant_py_reference.corpus",
    "tinyquant_py_reference.backend",
)


def _collect_cross_package_deps() -> dict[str, set[str]]:
    """Build a cross-package dependency map from sys.modules."""
    deps: dict[str, set[str]] = {p: set() for p in _PACKAGES}
    for mod_name, mod in sys.modules.items():
        owner = next((p for p in _PACKAGES if mod_name.startswith(p)), None)
        if owner is None or mod is None:
            continue
        for attr_name in dir(mod):
            attr_mod = getattr(getattr(mod, attr_name, None), "__module__", None)
            if attr_mod is not None:
                for target in _PACKAGES:
                    if target != owner and attr_mod.startswith(target):
                        deps[owner].add(target)
    return deps


class TestImportGraph:
    """The three bounded contexts form a DAG with no cycles."""

    def test_no_package_cycles(self) -> None:
        """Dependency graph codec -> corpus -> backend has no reverse edges."""
        with _clean_tinyquant_imports():
            importlib.import_module("tinyquant_py_reference.codec")
            importlib.import_module("tinyquant_py_reference.corpus")
            importlib.import_module("tinyquant_py_reference.backend")

            deps = _collect_cross_package_deps()

            # Forbidden reverse edges:
            assert (
                "tinyquant_py_reference.corpus"
                not in deps["tinyquant_py_reference.codec"]
            ), "cycle: codec depends on corpus"
            assert (
                "tinyquant_py_reference.backend"
                not in deps["tinyquant_py_reference.codec"]
            ), "cycle: codec depends on backend"
            assert (
                "tinyquant_py_reference.backend"
                not in deps["tinyquant_py_reference.corpus"]
            ), "cycle: corpus depends on backend"


class TestExports:
    """Each subpackage __init__ exports exactly its public API."""

    @pytest.mark.parametrize(
        "package_name",
        [
            "tinyquant_py_reference.codec",
            "tinyquant_py_reference.corpus",
            "tinyquant_py_reference.backend",
        ],
    )
    def test_init_exports_match_all(self, package_name: str) -> None:
        """Every name in __all__ is accessible; every public attr is in __all__."""
        mod = importlib.import_module(package_name)
        all_names: list[str] = getattr(mod, "__all__", [])
        assert all_names, f"{package_name} has no __all__"

        # Every name in __all__ must be accessible
        for name in all_names:
            assert hasattr(mod, name), (
                f"{package_name}.__all__ lists {name!r} but it is not accessible"
            )

        # Every public name in dir() should be in __all__
        public_attrs = {
            name
            for name in dir(mod)
            if not name.startswith("_")
            and not isinstance(getattr(mod, name, None), type(sys))
        }
        missing = public_attrs - set(all_names)
        assert not missing, f"{package_name} has public names not in __all__: {missing}"
