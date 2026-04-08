"""Private quantization helpers for the TinyQuant codec.

All functions are pure — no side effects, no hidden state.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:
    from numpy.typing import NDArray


def scalar_quantize(
    values: NDArray[np.float32],
    entries: NDArray[np.float32],
) -> NDArray[np.uint8]:
    """Map each value to the nearest codebook entry index.

    Args:
        values: 1-D float32 array of values to quantize.
        entries: Sorted 1-D float32 codebook entries.

    Returns:
        Array of uint8 indices into *entries*.
    """
    idx = np.searchsorted(entries, values, side="left")
    idx = np.clip(idx, 0, len(entries) - 1)
    left_idx = np.clip(idx - 1, 0, len(entries) - 1)
    left_dist = np.abs(values - entries[left_idx])
    right_dist = np.abs(values - entries[idx])
    result = np.where(left_dist < right_dist, left_idx, idx)
    return result.astype(np.uint8)


def scalar_dequantize(
    indices: NDArray[np.uint8],
    entries: NDArray[np.float32],
) -> NDArray[np.float32]:
    """Map indices back to representative FP32 values.

    Args:
        indices: Array of uint8 codebook indices.
        entries: Sorted 1-D float32 codebook entries.

    Returns:
        Array of float32 representative values.
    """
    return entries[indices]


def compute_residual(
    original: NDArray[np.float32],
    reconstructed: NDArray[np.float32],
) -> bytes:
    """Encode the difference between original and stage-1 reconstruction.

    Uses FP16 projection for compact storage at roughly half the FP32 cost.

    Args:
        original: Original float32 values (pre-quantization).
        reconstructed: Stage-1 dequantized float32 values.

    Returns:
        Residual correction encoded as bytes.
    """
    diff = original - reconstructed
    return diff.astype(np.float16).tobytes()


def apply_residual(
    reconstructed: NDArray[np.float32],
    residual: bytes,
) -> NDArray[np.float32]:
    """Apply residual correction to improve reconstruction fidelity.

    Args:
        reconstructed: Stage-1 dequantized float32 values.
        residual: Residual correction bytes produced by :func:`compute_residual`.

    Returns:
        Corrected float32 values.
    """
    correction = np.frombuffer(residual, dtype=np.float16).astype(np.float32)
    return reconstructed + correction
