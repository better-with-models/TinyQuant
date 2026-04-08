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


# ===========================================================================
# Search tests
# ===========================================================================


class TestSearch:
    """Tests for BruteForceBackend.search."""

    def test_search_returns_top_k_results(self) -> None:
        """Exactly top_k results when enough vectors exist."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(10))
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=3)
        assert len(results) == 3

    def test_search_results_ranked_by_similarity(self) -> None:
        """Results are in descending score order."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(10))
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=5)
        scores = [r.score for r in results]
        assert scores == sorted(scores, reverse=True)

    def test_search_with_fewer_vectors_than_top_k(self) -> None:
        """Returns all available vectors when fewer than top_k exist."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(2))
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=10)
        assert len(results) == 2

    def test_search_returns_search_result_objects(self) -> None:
        """Each element in results is a SearchResult."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(3))
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=2)
        for r in results:
            assert isinstance(r, SearchResult)

    def test_search_identical_vector_has_score_near_one(self) -> None:
        """Cosine similarity of a vector with itself is approximately 1.0."""
        vec = np.array([1.0, 2.0, 3.0, 4.0], dtype=np.float32)
        backend = BruteForceBackend()
        backend.ingest({"same": vec})
        results = backend.search(vec, top_k=1)
        assert results[0].score == pytest.approx(1.0, abs=1e-6)

    def test_search_orthogonal_vector_has_score_near_zero(self) -> None:
        """Cosine similarity of orthogonal vectors is approximately 0.0."""
        v1 = np.array([1.0, 0.0, 0.0, 0.0], dtype=np.float32)
        v2 = np.array([0.0, 1.0, 0.0, 0.0], dtype=np.float32)
        backend = BruteForceBackend()
        backend.ingest({"ortho": v2})
        results = backend.search(v1, top_k=1)
        assert results[0].score == pytest.approx(0.0, abs=1e-6)

    def test_search_empty_backend_returns_empty(self) -> None:
        """No vectors ingested yields an empty result list."""
        backend = BruteForceBackend()
        query = np.ones(4, dtype=np.float32)
        results = backend.search(query, top_k=5)
        assert results == []
