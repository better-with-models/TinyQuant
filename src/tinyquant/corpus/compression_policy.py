"""CompressionPolicy: enum governing corpus write-path behavior."""

from __future__ import annotations

import enum

import numpy as np


class CompressionPolicy(enum.Enum):
    """Policy that governs how vectors are stored in a corpus.

    A corpus's policy is immutable after the first vector is inserted.
    """

    COMPRESS = "compress"
    """Full codec pipeline: rotation, quantization, residual correction."""

    PASSTHROUGH = "passthrough"
    """Store FP32 vectors without any transformation."""

    FP16 = "fp16"
    """Downcast to FP16 without codec quantization."""

    def requires_codec(self) -> bool:
        """Return True only for the COMPRESS policy."""
        return self is CompressionPolicy.COMPRESS

    def storage_dtype(self) -> np.dtype[np.generic]:
        """Return the numpy dtype for stored vectors under this policy."""
        if self is CompressionPolicy.COMPRESS:
            return np.dtype(np.uint8)
        if self is CompressionPolicy.FP16:
            return np.dtype(np.float16)
        return np.dtype(np.float32)
