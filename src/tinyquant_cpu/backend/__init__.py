"""TinyQuant backend: search protocol and adapters (Rust-backed)."""

from __future__ import annotations

import sys

import tinyquant_cpu  # noqa: F401

_core_backend = sys.modules["tinyquant_cpu._core"].backend

BruteForceBackend = _core_backend.BruteForceBackend
SearchBackend = _core_backend.SearchBackend
SearchResult = _core_backend.SearchResult

__all__ = [
    "BruteForceBackend",
    "SearchBackend",
    "SearchResult",
]
