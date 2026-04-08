"""Unit tests for BruteForceBackend and SearchResult."""

from __future__ import annotations

import pytest

from tinyquant.backend.protocol import SearchResult


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
