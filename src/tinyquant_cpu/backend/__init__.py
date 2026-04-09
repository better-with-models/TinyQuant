"""TinyQuant backend: search protocol and adapter contracts."""

from tinyquant_cpu.backend.brute_force import BruteForceBackend
from tinyquant_cpu.backend.protocol import SearchBackend, SearchResult

__all__ = [
    "BruteForceBackend",
    "SearchBackend",
    "SearchResult",
]
