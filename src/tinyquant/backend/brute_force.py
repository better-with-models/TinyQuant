"""Brute-force search backend: exhaustive cosine similarity."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

    import numpy as np
    from numpy.typing import NDArray

    from tinyquant.backend.protocol import SearchResult


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
        raise NotImplementedError  # implemented in Task 4

    def remove(
        self,
        vector_ids: Sequence[str],
    ) -> None:
        """Remove vectors by ID. Unknown IDs are silently ignored.

        Args:
            vector_ids: IDs to remove.
        """
        raise NotImplementedError  # implemented in Task 5

    def clear(self) -> None:
        """Remove all stored vectors."""
        raise NotImplementedError  # implemented in Task 5
