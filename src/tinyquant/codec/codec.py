"""Codec: stateless domain service for vector compression and decompression."""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

from tinyquant.codec._errors import (
    CodebookIncompatibleError,
    ConfigMismatchError,
    DimensionMismatchError,
)
from tinyquant.codec._quantize import apply_residual, compute_residual
from tinyquant.codec.codebook import Codebook
from tinyquant.codec.compressed_vector import CompressedVector
from tinyquant.codec.rotation_matrix import RotationMatrix

if TYPE_CHECKING:
    from collections.abc import Sequence

    from numpy.typing import NDArray

    from tinyquant.codec.codec_config import CodecConfig


class Codec:
    """Stateless domain service that performs vector compression and decompression.

    Owns the full pipeline: rotation, quantization, residual correction,
    and their inverses. All behavior is determined by the ``CodecConfig``
    and ``Codebook`` passed to each method.
    """

    def compress(
        self,
        vector: NDArray[np.float32],
        config: CodecConfig,
        codebook: Codebook,
    ) -> CompressedVector:
        """Compress a single FP32 vector.

        Args:
            vector: 1-D float32 array of length ``config.dimension``.
            config: Codec configuration.
            codebook: Trained codebook matching *config.bit_width*.

        Returns:
            A :class:`CompressedVector` carrying quantized indices and
            optional residual correction.

        Raises:
            DimensionMismatchError: If ``len(vector) != config.dimension``.
            CodebookIncompatibleError: If the codebook was trained under a
                different bit width.
        """
        self._validate_compress_inputs(vector, config, codebook)

        rotation = RotationMatrix.from_config(config)
        rotated = rotation.apply(vector)
        indices = codebook.quantize(rotated)

        residual: bytes | None = None
        if config.residual_enabled:
            reconstructed = codebook.dequantize(indices)
            residual = compute_residual(rotated, reconstructed)

        return CompressedVector(
            indices=indices,
            residual=residual,
            config_hash=config.config_hash,
            dimension=config.dimension,
        )

    def decompress(
        self,
        compressed: CompressedVector,
        config: CodecConfig,
        codebook: Codebook,
    ) -> NDArray[np.float32]:
        """Decompress a :class:`CompressedVector` back to FP32.

        Args:
            compressed: Previously compressed vector.
            config: Codec configuration matching the one used to compress.
            codebook: Trained codebook matching *config.bit_width*.

        Returns:
            1-D float32 array of length ``config.dimension``.

        Raises:
            ConfigMismatchError: If the compressed vector's config hash
                does not match *config*.
            CodebookIncompatibleError: If the codebook was trained under a
                different bit width.
        """
        self._validate_decompress_inputs(compressed, config, codebook)

        rotation = RotationMatrix.from_config(config)
        values = codebook.dequantize(compressed.indices)

        if compressed.has_residual:
            values = apply_residual(values, compressed.residual)  # type: ignore[arg-type]

        return rotation.apply_inverse(values)

    def compress_batch(
        self,
        vectors: NDArray[np.float32],
        config: CodecConfig,
        codebook: Codebook,
    ) -> list[CompressedVector]:
        """Compress a batch of FP32 vectors.

        Args:
            vectors: 2-D float32 array of shape ``(n, config.dimension)``.
            config: Codec configuration.
            codebook: Trained codebook matching *config.bit_width*.

        Returns:
            List of *n* :class:`CompressedVector` instances, each identical
            to what :meth:`compress` would produce individually.
        """
        return [
            self.compress(vectors[i], config, codebook) for i in range(len(vectors))
        ]

    def decompress_batch(
        self,
        compressed: Sequence[CompressedVector],
        config: CodecConfig,
        codebook: Codebook,
    ) -> NDArray[np.float32]:
        """Decompress a batch of compressed vectors.

        Args:
            compressed: Sequence of compressed vectors.
            config: Codec configuration.
            codebook: Trained codebook matching *config.bit_width*.

        Returns:
            2-D float32 array of shape ``(n, config.dimension)``.
        """
        results = [self.decompress(cv, config, codebook) for cv in compressed]
        return np.stack(results)

    def build_codebook(
        self,
        training_vectors: NDArray[np.float32],
        config: CodecConfig,
    ) -> Codebook:
        """Train a codebook from representative vectors.

        Args:
            training_vectors: 2-D float32 array of training data.
            config: Codec configuration determining the bit width.

        Returns:
            A frozen :class:`Codebook` with ``2 ** config.bit_width`` entries.

        Raises:
            ValueError: If training data has insufficient unique values.
        """
        return Codebook.train(training_vectors, config)

    def build_rotation(
        self,
        config: CodecConfig,
    ) -> RotationMatrix:
        """Generate a rotation matrix from a codec configuration.

        Args:
            config: Codec configuration determining seed and dimension.

        Returns:
            A deterministic orthogonal :class:`RotationMatrix`.
        """
        return RotationMatrix.from_config(config)

    # ------------------------------------------------------------------
    # Private validation helpers
    # ------------------------------------------------------------------

    @staticmethod
    def _validate_compress_inputs(
        vector: NDArray[np.float32],
        config: CodecConfig,
        codebook: Codebook,
    ) -> None:
        if len(vector) != config.dimension:
            msg = (
                f"vector length {len(vector)} does not match "
                f"config dimension {config.dimension}"
            )
            raise DimensionMismatchError(msg)
        if codebook.bit_width != config.bit_width:
            msg = (
                f"codebook bit_width {codebook.bit_width} does not match "
                f"config bit_width {config.bit_width}"
            )
            raise CodebookIncompatibleError(msg)

    @staticmethod
    def _validate_decompress_inputs(
        compressed: CompressedVector,
        config: CodecConfig,
        codebook: Codebook,
    ) -> None:
        if compressed.config_hash != config.config_hash:
            msg = (
                f"compressed config_hash {compressed.config_hash!r} does not match "
                f"config hash {config.config_hash!r}"
            )
            raise ConfigMismatchError(msg)
        if codebook.bit_width != config.bit_width:
            msg = (
                f"codebook bit_width {codebook.bit_width} does not match "
                f"config bit_width {config.bit_width}"
            )
            raise CodebookIncompatibleError(msg)


# ---------------------------------------------------------------------------
# Module-level convenience functions
# ---------------------------------------------------------------------------

_DEFAULT_CODEC = Codec()


def compress(
    vector: NDArray[np.float32],
    config: CodecConfig,
    codebook: Codebook,
) -> CompressedVector:
    """Compress a single FP32 vector using the default codec instance.

    See :meth:`Codec.compress` for full documentation.
    """
    return _DEFAULT_CODEC.compress(vector, config, codebook)


def decompress(
    compressed: CompressedVector,
    config: CodecConfig,
    codebook: Codebook,
) -> NDArray[np.float32]:
    """Decompress a compressed vector using the default codec instance.

    See :meth:`Codec.decompress` for full documentation.
    """
    return _DEFAULT_CODEC.decompress(compressed, config, codebook)
