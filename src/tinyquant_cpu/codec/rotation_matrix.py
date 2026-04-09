"""RotationMatrix: deterministic orthogonal transform for vector preconditioning."""

from __future__ import annotations

import functools
from dataclasses import dataclass
from typing import TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:
    from numpy.typing import NDArray

    from tinyquant_cpu.codec.codec_config import CodecConfig


@dataclass(frozen=True)
class RotationMatrix:
    """Deterministically generated orthogonal matrix for vector preconditioning.

    Preconditions vector coordinates before quantization, eliminating
    per-block normalization overhead.
    """

    matrix: NDArray[np.float64]
    """Orthogonal matrix of shape ``(dimension, dimension)``."""

    seed: int
    """The seed used to generate this matrix."""

    dimension: int
    """Matrix row/column count; matches ``CodecConfig.dimension``."""

    def __post_init__(self) -> None:
        """Validate shape and make the matrix array immutable."""
        if self.matrix.shape != (self.dimension, self.dimension):
            msg = (
                f"matrix shape must be ({self.dimension}, {self.dimension}), "
                f"got {self.matrix.shape}"
            )
            raise ValueError(msg)
        # Freeze the underlying array to enforce immutability.
        frozen = self.matrix.copy()
        frozen.flags.writeable = False
        object.__setattr__(self, "matrix", frozen)

    @classmethod
    def from_config(cls, config: CodecConfig) -> RotationMatrix:
        """Generate a rotation matrix deterministically from a codec config.

        Results are cached by ``(seed, dimension)`` because the QR
        decomposition of a 768x768 matrix is expensive.
        """
        return cls._cached_build(config.seed, config.dimension)

    @staticmethod
    @functools.lru_cache(maxsize=8)
    def _cached_build(seed: int, dimension: int) -> RotationMatrix:
        """Build and cache the rotation matrix for a ``(seed, dimension)`` pair.

        Uses QR decomposition of a seeded random matrix to produce a
        uniformly distributed orthogonal matrix (Haar measure).
        """
        rng = np.random.default_rng(seed)
        random_matrix = rng.standard_normal((dimension, dimension))
        q, r = np.linalg.qr(random_matrix)
        # Haar-measure sign correction: ensure deterministic sign convention.
        d = np.diag(r)
        ph = np.sign(d)
        q = q * ph[np.newaxis, :]
        return RotationMatrix(matrix=q, seed=seed, dimension=dimension)

    def apply(self, vector: NDArray[np.float32]) -> NDArray[np.float32]:
        """Rotate a vector: ``R @ vector``.

        Args:
            vector: 1-D float32 array of length ``dimension``.

        Returns:
            Rotated vector as float32.

        Raises:
            ValueError: If the vector dimension does not match.
        """
        if vector.shape != (self.dimension,):
            msg = f"vector dimension must be {self.dimension}, got shape {vector.shape}"
            raise ValueError(msg)
        return (self.matrix @ vector.astype(np.float64)).astype(np.float32)

    def apply_inverse(self, rotated: NDArray[np.float32]) -> NDArray[np.float32]:
        """Inverse rotation: ``R.T @ rotated`` (valid because R is orthogonal).

        Args:
            rotated: 1-D float32 array of length ``dimension``.

        Returns:
            Recovered vector as float32.

        Raises:
            ValueError: If the vector dimension does not match.
        """
        if rotated.shape != (self.dimension,):
            msg = (
                f"vector dimension must be {self.dimension}, got shape {rotated.shape}"
            )
            raise ValueError(msg)
        return (self.matrix.T @ rotated.astype(np.float64)).astype(np.float32)

    def verify_orthogonality(self, tol: float = 1e-6) -> bool:
        """Check that ``R @ R.T`` approximates the identity within *tol*.

        Args:
            tol: Absolute tolerance for the orthogonality check.

        Returns:
            ``True`` if the matrix is orthogonal within tolerance.
        """
        product = self.matrix @ self.matrix.T
        return bool(np.allclose(product, np.eye(self.dimension), atol=tol))
