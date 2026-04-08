"""Domain events for the TinyQuant corpus bounded context."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from datetime import datetime

    from tinyquant.codec.codec_config import CodecConfig
    from tinyquant.corpus.compression_policy import CompressionPolicy


@dataclass(frozen=True)
class CorpusCreated:
    """Raised when a new corpus is initialized."""

    corpus_id: str
    """Unique corpus identity."""

    codec_config: CodecConfig
    """Frozen configuration."""

    compression_policy: CompressionPolicy
    """Selected write-path policy."""

    timestamp: datetime
    """UTC creation time."""


@dataclass(frozen=True)
class VectorsInserted:
    """Raised after one or more vectors are successfully stored."""

    corpus_id: str
    """Target corpus."""

    vector_ids: tuple[str, ...]
    """IDs of inserted vectors (tuple for immutability)."""

    count: int
    """Number of vectors inserted."""

    timestamp: datetime
    """UTC insertion time."""


@dataclass(frozen=True)
class CorpusDecompressed:
    """Raised when a batch decompression produces FP32 vectors."""

    corpus_id: str
    """Source corpus."""

    vector_count: int
    """Number of vectors decompressed."""

    timestamp: datetime
    """UTC decompression time."""


@dataclass(frozen=True)
class CompressionPolicyViolationDetected:
    """Raised when an operation violates corpus configuration or policy."""

    corpus_id: str
    """Affected corpus."""

    violation_type: str
    """Category: 'config_mismatch', 'policy_conflict', 'duplicate_id'."""

    detail: str
    """Human-readable description."""

    timestamp: datetime
    """UTC detection time."""
