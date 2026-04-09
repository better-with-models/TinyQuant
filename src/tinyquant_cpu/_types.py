"""Shared type aliases for TinyQuant internals."""

from __future__ import annotations

import numpy as np
from numpy.typing import NDArray

VectorId = str
"""Semantic alias for vector identity strings."""

ConfigHash = str
"""Deterministic hash identifying a CodecConfig."""

CorpusId = str
"""Semantic alias for corpus identity strings."""

Vector = NDArray[np.float32]
"""1-D FP32 embedding array."""

VectorBatch = NDArray[np.float32]
"""2-D FP32 array of shape ``(n, dimension)``."""

__all__: list[str] = [
    "ConfigHash",
    "CorpusId",
    "Vector",
    "VectorBatch",
    "VectorId",
]
