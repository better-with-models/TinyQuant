"""Integration tests for Corpus -> Backend protocol handoff (IB-02)."""

from __future__ import annotations

import numpy as np
from numpy.typing import NDArray

from tinyquant.backend.brute_force import BruteForceBackend
from tinyquant.corpus.corpus import Corpus


class TestCorpusBackendIntegration:
    """IB-02: decompress_all() output feeds BruteForceBackend correctly."""

    def test_decompress_all_produces_valid_backend_input(
        self,
        sample_corpus: Corpus,
        brute_force_backend: BruteForceBackend,
    ) -> None:
        """Backend accepts decompress_all() output without errors."""
        decompressed = sample_corpus.decompress_all()
        brute_force_backend.ingest(decompressed)
        assert brute_force_backend.count == 100

    def test_decompressed_vectors_have_correct_dimension(
        self,
        sample_corpus: Corpus,
    ) -> None:
        """Every decompressed vector has expected shape and dtype."""
        decompressed = sample_corpus.decompress_all()
        for vec in decompressed.values():
            assert vec.dtype == np.float32
            assert vec.shape == (64,)

    def test_backend_search_after_corpus_decompress(
        self,
        sample_corpus: Corpus,
        brute_force_backend: BruteForceBackend,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Full read path: decompress -> ingest -> search returns ranked results."""
        decompressed = sample_corpus.decompress_all()
        brute_force_backend.ingest(decompressed)

        results = brute_force_backend.search(sample_vectors[0], top_k=5)
        assert len(results) == 5
        assert all(r.vector_id.startswith("vec-") for r in results)
        # Scores should be in descending order
        scores = [r.score for r in results]
        assert scores == sorted(scores, reverse=True)
        # Top match should be reasonably similar to the query
        assert results[0].score > 0.5

    def test_vector_ids_preserved_through_handoff(
        self,
        sample_corpus: Corpus,
        brute_force_backend: BruteForceBackend,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Vector IDs survive corpus -> backend handoff."""
        decompressed = sample_corpus.decompress_all()
        decompressed_ids = frozenset(decompressed.keys())
        corpus_ids = sample_corpus.vector_ids
        assert decompressed_ids == corpus_ids

        brute_force_backend.ingest(decompressed)
        results = brute_force_backend.search(sample_vectors[0], top_k=100)
        result_ids = frozenset(r.vector_id for r in results)
        assert result_ids == corpus_ids
