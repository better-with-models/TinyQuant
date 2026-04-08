"""Search protocol and result value object."""

from __future__ import annotations

from dataclasses import dataclass


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
