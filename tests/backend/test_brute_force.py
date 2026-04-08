"""Unit tests for BruteForceBackend and SearchResult."""

from __future__ import annotations

import numpy as np
import pytest
from numpy.typing import NDArray

from tinyquant.backend.brute_force import BruteForceBackend
from tinyquant.backend.protocol import SearchResult


# ===========================================================================
# Helpers
# ===========================================================================


def _make_vectors(
    n: int,
    dim: int = 8,
    seed: int = 42,
) -> dict[str, NDArray[np.float32]]:
    """Generate n random FP32 vectors keyed by 'vec-000' .. 'vec-{n-1}'."""
    rng = np.random.default_rng(seed)
    return {
        f"vec-{i:03d}": rng.standard_normal(dim).astype(np.float32)
        for i in range(n)
    }


# ===========================================================================
# SearchResult tests
# ===========================================================================


class TestSearchResult:
    """Tests for SearchResult construction and ordering."""

    def test_construction(self) -> None:
        """SearchResult stores vector_id and score."""
        result = SearchResult(vector_id="vec-001", score=0.95)
        assert result.vector_id == "vec-001"
        assert result.score == pytest.approx(0.95)

    def test_ordering_by_score_descending(self) -> None:
        """Sorted list ranks higher scores first."""
        r1 = SearchResult(vector_id="a", score=0.3)
        r2 = SearchResult(vector_id="b", score=0.9)
        r3 = SearchResult(vector_id="c", score=0.6)
        ranked = sorted([r1, r2, r3])
        assert [r.vector_id for r in ranked] == ["b", "c", "a"]

    def test_frozen(self) -> None:
        """SearchResult is immutable."""
        result = SearchResult(vector_id="vec-001", score=0.5)
        with pytest.raises(AttributeError):
            result.score = 0.99  # type: ignore[misc]


# ===========================================================================
# Ingest tests
# ===========================================================================


class TestIngest:
    """Tests for BruteForceBackend.ingest."""

    def test_ingest_stores_vectors(self) -> None:
        """Count increases after ingestion."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(5))
        assert backend.count == 5

    def test_ingest_duplicate_id_overwrites(self) -> None:
        """Second ingest with same ID replaces the vector, count unchanged."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(3))
        backend.ingest({"vec-000": np.ones(8, dtype=np.float32)})
        assert backend.count == 3

    def test_ingest_empty_dict_is_noop(self) -> None:
        """Ingesting empty dict causes no error and no count change."""
        backend = BruteForceBackend()
        backend.ingest({})
        assert backend.count == 0
