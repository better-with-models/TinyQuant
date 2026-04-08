"""CompressedVector: immutable codec output value object."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import numpy as np
    from numpy.typing import NDArray

    from tinyquant._types import ConfigHash


@dataclass(frozen=True)
class CompressedVector:
    """Immutable output of a codec compression pass.

    Carries quantized indices, optional residual data, and a config hash
    linking it to the originating configuration.
    """

    indices: NDArray[np.uint8]
    """One quantized index per dimension."""

    residual: bytes | None
    """Residual correction data, or ``None`` if residual was disabled."""

    config_hash: ConfigHash
    """Fingerprint of the ``CodecConfig`` that produced this vector."""

    dimension: int
    """Length of ``indices``; stored explicitly for fast validation."""

    def __post_init__(self) -> None:
        """Validate index/dimension consistency and freeze the indices array."""
        if len(self.indices) != self.dimension:
            msg = (
                f"indices length ({len(self.indices)}) must match "
                f"dimension ({self.dimension})"
            )
            raise ValueError(msg)
        frozen = self.indices.copy()
        frozen.flags.writeable = False
        object.__setattr__(self, "indices", frozen)

    @property
    def has_residual(self) -> bool:
        """Whether this vector carries residual correction data."""
        return self.residual is not None

    @property
    def size_bytes(self) -> int:
        """Approximate storage footprint in bytes."""
        return self.indices.nbytes + (len(self.residual) if self.residual else 0)

    def to_bytes(self) -> bytes:
        """Serialize to a compact binary representation.

        Raises:
            NotImplementedError: Serialization will be implemented in Phase 6.
        """
        raise NotImplementedError("Serialization will be implemented in Phase 6")

    @classmethod
    def from_bytes(cls, data: bytes) -> CompressedVector:
        """Deserialize from a compact binary representation.

        Args:
            data: Binary data to deserialize.

        Raises:
            NotImplementedError: Deserialization will be implemented in Phase 6.
        """
        raise NotImplementedError("Deserialization will be implemented in Phase 6")
