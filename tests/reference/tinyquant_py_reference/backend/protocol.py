"""Search protocol and result value object."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    from collections.abc import Sequence

    import numpy as np
    from numpy.typing import NDArray


@dataclass(frozen=True)
class SearchResult:
    """Immutable value object representing a single ranked search result.

    Attributes:
        vector_id: Identity of the matched vector.
        score: Similarity score (higher is more similar).
    """

    vector_id: str
    score: float

    def __lt__(self, other: object) -> bool:
        """Order by score descending — higher scores sort first."""
        if not isinstance(other, SearchResult):
            return NotImplemented
        return self.score > other.score


class SearchBackend(Protocol):
    """Contract for vector search backends.

    Backends receive FP32 vectors only — never compressed representations.
    Any class with matching method signatures satisfies this protocol
    (structural typing, no inheritance needed).
    """

    def search(
        self,
        query: NDArray[np.float32],
        top_k: int,
    ) -> list[SearchResult]:
        """Find the top_k most similar vectors to query.

        Args:
            query: 1-D FP32 array matching ingested vector dimension.
            top_k: Maximum number of results to return. Must be > 0.

        Returns:
            Up to top_k results ranked by descending similarity.
        """
        ...

    def ingest(
        self,
        vectors: dict[str, NDArray[np.float32]],
    ) -> None:
        """Accept decompressed FP32 vectors for indexing.

        Args:
            vectors: Mapping of vector_id to 1-D FP32 array.
                     All vectors must have the same dimension.
        """
        ...

    def remove(
        self,
        vector_ids: Sequence[str],
    ) -> None:
        """Remove vectors from the index by ID.

        Args:
            vector_ids: IDs to remove. Unknown IDs are silently ignored.
        """
        ...
