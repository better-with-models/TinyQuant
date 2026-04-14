"""TinyQuant backend: search protocol and adapter contracts."""

from tinyquant_py_reference.backend.brute_force import BruteForceBackend
from tinyquant_py_reference.backend.protocol import SearchBackend, SearchResult

__all__ = [
    "BruteForceBackend",
    "SearchBackend",
    "SearchResult",
]
