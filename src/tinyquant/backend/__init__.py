"""TinyQuant backend: search protocol and adapter contracts."""

from tinyquant.backend.brute_force import BruteForceBackend
from tinyquant.backend.protocol import SearchBackend, SearchResult

__all__ = [
    "BruteForceBackend",
    "SearchBackend",
    "SearchResult",
]
