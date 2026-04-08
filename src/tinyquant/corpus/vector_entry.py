"""VectorEntry: entity representing a stored vector in a corpus."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from datetime import datetime

    from tinyquant._types import ConfigHash
    from tinyquant.codec.compressed_vector import CompressedVector


class VectorEntry:
    """Entity with identity by ``vector_id`` and lifecycle within the aggregate.

    Equality and hashing are by ``vector_id`` only, making entries usable
    in sets and as dict keys.
    """

    __slots__ = ("compressed", "inserted_at", "metadata", "vector_id")

    def __init__(
        self,
        vector_id: str,
        compressed: CompressedVector,
        inserted_at: datetime,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        """Initialize a vector entry.

        Args:
            vector_id: Unique identity within the corpus.
            compressed: The stored compressed representation.
            inserted_at: UTC timestamp of insertion.
            metadata: Optional per-vector metadata.
        """
        self.vector_id = vector_id
        self.compressed = compressed
        self.inserted_at = inserted_at
        self.metadata = metadata

    @property
    def config_hash(self) -> ConfigHash:
        """Delegate to ``compressed.config_hash``."""
        return self.compressed.config_hash

    @property
    def dimension(self) -> int:
        """Delegate to ``compressed.dimension``."""
        return self.compressed.dimension

    @property
    def has_residual(self) -> bool:
        """Delegate to ``compressed.has_residual``."""
        return self.compressed.has_residual

    def __eq__(self, other: object) -> bool:
        """Identity equality by ``vector_id``."""
        if not isinstance(other, VectorEntry):
            return NotImplemented
        return self.vector_id == other.vector_id

    def __hash__(self) -> int:
        """Hash by ``vector_id``."""
        return hash(self.vector_id)

    def __repr__(self) -> str:
        """Return developer-friendly string representation."""
        return (
            f"VectorEntry(vector_id={self.vector_id!r}, "
            f"dimension={self.dimension}, "
            f"has_residual={self.has_residual})"
        )
