"""Tests for corpus domain events."""

from __future__ import annotations

from dataclasses import FrozenInstanceError
from datetime import UTC, datetime

import pytest

from tinyquant.codec.codec_config import CodecConfig
from tinyquant.corpus.compression_policy import CompressionPolicy
from tinyquant.corpus.events import (
    CompressionPolicyViolationDetected,
    CorpusCreated,
    CorpusDecompressed,
    VectorsInserted,
)


class TestCorpusCreated:
    """Tests for CorpusCreated event."""

    def test_corpus_created_fields(self) -> None:
        """All fields populated correctly."""
        config = CodecConfig(bit_width=4, seed=42, dimension=64)
        now = datetime.now(UTC)
        event = CorpusCreated(
            corpus_id="c1",
            codec_config=config,
            compression_policy=CompressionPolicy.COMPRESS,
            timestamp=now,
        )
        assert event.corpus_id == "c1"
        assert event.codec_config is config
        assert event.compression_policy is CompressionPolicy.COMPRESS
        assert event.timestamp == now

    def test_corpus_created_frozen(self) -> None:
        """Assignment to fields raises error."""
        config = CodecConfig(bit_width=4, seed=42, dimension=64)
        now = datetime.now(UTC)
        event = CorpusCreated("c1", config, CompressionPolicy.COMPRESS, now)
        with pytest.raises(FrozenInstanceError):
            event.corpus_id = "c2"  # type: ignore[misc]

    def test_corpus_created_timestamp_is_utc(self) -> None:
        """Timestamp is timezone-aware UTC."""
        config = CodecConfig(bit_width=4, seed=42, dimension=64)
        now = datetime.now(UTC)
        event = CorpusCreated("c1", config, CompressionPolicy.COMPRESS, now)
        assert event.timestamp.tzinfo is not None


class TestVectorsInserted:
    """Tests for VectorsInserted event."""

    def test_vectors_inserted_fields(self) -> None:
        """vector_ids is a tuple and count matches length."""
        now = datetime.now(UTC)
        event = VectorsInserted(
            corpus_id="c1",
            vector_ids=("v1", "v2", "v3"),
            count=3,
            timestamp=now,
        )
        assert event.corpus_id == "c1"
        assert event.vector_ids == ("v1", "v2", "v3")
        assert event.count == 3

    def test_vectors_inserted_frozen(self) -> None:
        """Event is immutable."""
        event = VectorsInserted("c1", ("v1",), 1, datetime.now(UTC))
        with pytest.raises(FrozenInstanceError):
            event.corpus_id = "c2"  # type: ignore[misc]

    def test_vectors_inserted_ids_are_tuple(self) -> None:
        """vector_ids uses tuple, not list."""
        event = VectorsInserted("c1", ("v1", "v2"), 2, datetime.now(UTC))
        assert isinstance(event.vector_ids, tuple)


class TestCorpusDecompressed:
    """Tests for CorpusDecompressed event."""

    def test_corpus_decompressed_fields(self) -> None:
        """vector_count and corpus_id populated."""
        now = datetime.now(UTC)
        event = CorpusDecompressed("c1", vector_count=50, timestamp=now)
        assert event.corpus_id == "c1"
        assert event.vector_count == 50
        assert event.timestamp == now

    def test_corpus_decompressed_frozen(self) -> None:
        """Event is immutable."""
        event = CorpusDecompressed("c1", 10, datetime.now(UTC))
        with pytest.raises(FrozenInstanceError):
            event.vector_count = 99  # type: ignore[misc]


class TestCompressionPolicyViolationDetected:
    """Tests for CompressionPolicyViolationDetected event."""

    def test_violation_detected_fields(self) -> None:
        """All fields populated correctly."""
        now = datetime.now(UTC)
        event = CompressionPolicyViolationDetected(
            corpus_id="c1",
            violation_type="duplicate_id",
            detail="vec-001 already exists",
            timestamp=now,
        )
        assert event.corpus_id == "c1"
        assert event.violation_type == "duplicate_id"
        assert event.detail == "vec-001 already exists"

    def test_violation_detected_frozen(self) -> None:
        """Event is immutable."""
        event = CompressionPolicyViolationDetected(
            "c1", "config_mismatch", "hash differs", datetime.now(UTC)
        )
        with pytest.raises(FrozenInstanceError):
            event.detail = "changed"  # type: ignore[misc]
