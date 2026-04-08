"""Tests for VectorEntry entity."""

from __future__ import annotations

from datetime import UTC, datetime

import numpy as np

from tinyquant.codec.compressed_vector import CompressedVector
from tinyquant.corpus.vector_entry import VectorEntry


def _make_compressed(config_hash: str = "abc123", dim: int = 4) -> CompressedVector:
    """Helper to create a minimal CompressedVector."""
    return CompressedVector(
        indices=np.zeros(dim, dtype=np.uint8),
        residual=b"\x00\x01",
        config_hash=config_hash,
        dimension=dim,
        bit_width=4,
    )


class TestConstruction:
    """Tests for VectorEntry creation."""

    def test_construction_with_valid_fields(self) -> None:
        """All fields accessible after creation."""
        cv = _make_compressed()
        now = datetime.now(UTC)
        entry = VectorEntry("vec-001", cv, now, metadata={"source": "test"})
        assert entry.vector_id == "vec-001"
        assert entry.compressed is cv
        assert entry.inserted_at == now
        assert entry.metadata == {"source": "test"}

    def test_metadata_defaults_to_none(self) -> None:
        """Metadata defaults to None when not provided."""
        entry = VectorEntry("vec-001", _make_compressed(), datetime.now(UTC))
        assert entry.metadata is None


class TestDelegation:
    """Tests for property delegation to CompressedVector."""

    def test_config_hash_delegates_to_compressed(self) -> None:
        """config_hash matches compressed.config_hash."""
        cv = _make_compressed(config_hash="hash42")
        entry = VectorEntry("v1", cv, datetime.now(UTC))
        assert entry.config_hash == "hash42"

    def test_dimension_delegates_to_compressed(self) -> None:
        """Dimension matches compressed.dimension."""
        cv = _make_compressed(dim=8)
        entry = VectorEntry("v1", cv, datetime.now(UTC))
        assert entry.dimension == 8

    def test_has_residual_delegates_to_compressed(self) -> None:
        """has_residual matches compressed.has_residual."""
        cv = _make_compressed()
        entry = VectorEntry("v1", cv, datetime.now(UTC))
        assert entry.has_residual is True


class TestIdentityEquality:
    """Tests for equality and hashing by vector_id."""

    def test_equality_by_vector_id(self) -> None:
        """Same vector_id means equal, regardless of other fields."""
        a = VectorEntry("v1", _make_compressed(), datetime.now(UTC))
        b = VectorEntry("v1", _make_compressed(), datetime.now(UTC), {"k": "v"})
        assert a == b

    def test_inequality_by_vector_id(self) -> None:
        """Different vector_id means not equal."""
        a = VectorEntry("v1", _make_compressed(), datetime.now(UTC))
        b = VectorEntry("v2", _make_compressed(), datetime.now(UTC))
        assert a != b

    def test_hash_by_vector_id(self) -> None:
        """Same vector_id produces same hash for set/dict use."""
        a = VectorEntry("v1", _make_compressed(), datetime.now(UTC))
        b = VectorEntry("v1", _make_compressed(), datetime.now(UTC))
        assert hash(a) == hash(b)
        assert len({a, b}) == 1

    def test_metadata_is_mutable(self) -> None:
        """Metadata dict can be updated after construction."""
        entry = VectorEntry("v1", _make_compressed(), datetime.now(UTC), {"a": 1})
        assert entry.metadata is not None
        entry.metadata["b"] = 2
        assert entry.metadata == {"a": 1, "b": 2}
