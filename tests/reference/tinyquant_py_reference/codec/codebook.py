"""Codebook: immutable quantization lookup table for TinyQuant codec."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:
    from numpy.typing import NDArray
    from tinyquant_py_reference.codec.codec_config import CodecConfig


@dataclass(frozen=True)
class Codebook:
    """Immutable lookup table mapping quantized indices to representative FP32 values.

    Trained from sample data and frozen before use.
    """

    entries: NDArray[np.float32]
    """Sorted codebook entries of shape ``(num_entries,)``."""

    bit_width: int
    """Quantization bit width; matches the originating ``CodecConfig.bit_width``."""

    def __post_init__(self) -> None:
        """Validate entry count and freeze the entries array."""
        expected = 2**self.bit_width
        if len(self.entries) != expected:
            msg = (
                f"entries length must be {expected} for bit_width={self.bit_width}, "
                f"got {len(self.entries)}"
            )
            raise ValueError(msg)
        frozen = self.entries.copy()
        frozen.flags.writeable = False
        object.__setattr__(self, "entries", frozen)

    @property
    def num_entries(self) -> int:
        """Number of codebook entries: ``len(self.entries)``."""
        return len(self.entries)

    @classmethod
    def train(
        cls,
        vectors: NDArray[np.float32],
        config: CodecConfig,
    ) -> Codebook:
        """Train a codebook from representative vectors.

        Uses uniform quantile estimation.

        Args:
            vectors: 2-D float32 array of training vectors.
            config: Codec configuration determining the bit width.

        Returns:
            A frozen Codebook with ``2 ** config.bit_width`` entries.

        Raises:
            ValueError: If training data has insufficient unique values.
        """
        num_entries = config.num_codebook_entries
        flat = vectors.flatten().astype(np.float64)
        quantiles = np.linspace(0, 1, num_entries)
        entries = np.quantile(flat, quantiles).astype(np.float32)
        entries = np.sort(entries)
        if len(np.unique(entries)) < num_entries:
            msg = (
                f"Insufficient unique training data to produce {num_entries} "
                f"distinct codebook entries"
            )
            raise ValueError(msg)
        return cls(entries=entries, bit_width=config.bit_width)

    def quantize(self, values: NDArray[np.float32]) -> NDArray[np.uint8]:
        """Map continuous values to nearest codebook indices.

        Args:
            values: 1-D float32 array of values to quantize.

        Returns:
            Array of uint8 indices into the codebook.
        """
        # searchsorted gives the insertion point; compare with left neighbor.
        idx = np.searchsorted(self.entries, values, side="left")
        idx = np.clip(idx, 0, self.num_entries - 1)
        # Check if the left neighbor (idx - 1) is closer.
        left_idx = np.clip(idx - 1, 0, self.num_entries - 1)
        left_dist = np.abs(values - self.entries[left_idx])
        right_dist = np.abs(values - self.entries[idx])
        result = np.where(left_dist < right_dist, left_idx, idx)
        return result.astype(np.uint8)

    def dequantize(self, indices: NDArray[np.uint8]) -> NDArray[np.float32]:
        """Map indices back to representative FP32 values.

        Args:
            indices: Array of uint8 codebook indices.

        Returns:
            Array of float32 representative values.

        Raises:
            ValueError: If any index is out of range.
        """
        if np.any(indices >= self.num_entries):
            msg = f"Index out of range: all indices must be < {self.num_entries}"
            raise ValueError(msg)
        return self.entries[indices]
