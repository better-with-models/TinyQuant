"""Brute-force search backend: exhaustive cosine similarity."""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

from tinyquant.backend.protocol import SearchResult

if TYPE_CHECKING:
    from collections.abc import Sequence

    from numpy.typing import NDArray


class BruteForceBackend:
    """Reference SearchBackend using exhaustive cosine similarity.

    Intended for testing, calibration, and small-corpus use cases.
    Complexity: O(n * d) per search call.
    """

    def __init__(self) -> None:
        """Initialize with an empty vector store."""
        self._vectors: dict[str, NDArray[np.float32]] = {}

    @property
    def count(self) -> int:
        """Number of vectors currently stored."""
        return len(self._vectors)

    def ingest(
        self,
        vectors: dict[str, NDArray[np.float32]],
    ) -> None:
        """Add or replace vectors in the in-memory store.

        Args:
            vectors: Mapping of vector_id to 1-D FP32 array.
        """
        self._vectors.update(vectors)

    def search(
        self,
        query: NDArray[np.float32],
        top_k: int,
    ) -> list[SearchResult]:
        """Compute cosine similarity against all stored vectors.

        Args:
            query: 1-D FP32 query vector.
            top_k: Maximum results to return.

        Returns:
            Up to top_k results ranked by descending cosine similarity.
        """
        if not self._vectors:
            return []

        query_norm = float(np.linalg.norm(query))
        scored: list[SearchResult] = []
        for vid, vec in self._vectors.items():
            vec_norm = float(np.linalg.norm(vec))
            if query_norm == 0.0 or vec_norm == 0.0:
                score = 0.0
            else:
                score = float(np.dot(query, vec) / (query_norm * vec_norm))
            scored.append(SearchResult(vector_id=vid, score=score))

        scored.sort()
        return scored[:top_k]

    def remove(
        self,
        vector_ids: Sequence[str],
    ) -> None:
        """Remove vectors by ID. Unknown IDs are silently ignored.

        Args:
            vector_ids: IDs to remove.
        """
        for vid in vector_ids:
            self._vectors.pop(vid, None)

    def clear(self) -> None:
        """Remove all stored vectors."""
        self._vectors.clear()
