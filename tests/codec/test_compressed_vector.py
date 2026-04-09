"""Unit tests for CompressedVector value object."""

from __future__ import annotations

import dataclasses

import numpy as np
import pytest

from tinyquant_cpu.codec.compressed_vector import CompressedVector

# ---------------------------------------------------------------------------
# Construction and validation
# ---------------------------------------------------------------------------


class TestConstruction:
    """Tests for CompressedVector construction and validation."""

    def test_valid_compressed_vector_creates(self) -> None:
        """Matching dimension and indices length succeeds."""
        indices = np.array([3, 7, 1, 15], dtype=np.uint8)
        cv = CompressedVector(
            indices=indices,
            residual=None,
            config_hash="abc123",
            dimension=4,
            bit_width=4,
        )
        assert cv.dimension == 4
        assert cv.config_hash == "abc123"

    def test_dimension_mismatch_raises(self) -> None:
        """len(indices) != dimension raises ValueError."""
        indices = np.array([3, 7, 1], dtype=np.uint8)
        with pytest.raises(ValueError, match="dimension"):
            CompressedVector(
                indices=indices,
                residual=None,
                config_hash="abc123",
                dimension=4,
                bit_width=4,
            )

    def test_residual_none_when_disabled(self) -> None:
        """Residual field is None when correction was off."""
        cv = CompressedVector(
            indices=np.array([0, 1], dtype=np.uint8),
            residual=None,
            config_hash="abc123",
            dimension=2,
            bit_width=4,
        )
        assert cv.residual is None

    def test_residual_present_when_enabled(self) -> None:
        """Residual field is non-empty bytes when enabled."""
        residual_data = b"\x01\x02\x03\x04"
        cv = CompressedVector(
            indices=np.array([0, 1], dtype=np.uint8),
            residual=residual_data,
            config_hash="abc123",
            dimension=2,
            bit_width=4,
        )
        assert cv.residual == residual_data

    def test_invalid_bit_width_raises(self) -> None:
        """bit_width outside {2, 4, 8} raises ValueError."""
        with pytest.raises(ValueError, match="bit_width"):
            CompressedVector(
                indices=np.array([0], dtype=np.uint8),
                residual=None,
                config_hash="abc",
                dimension=1,
                bit_width=3,
            )


# ---------------------------------------------------------------------------
# Properties
# ---------------------------------------------------------------------------


class TestProperties:
    """Tests for computed properties."""

    def test_has_residual_true(self) -> None:
        """has_residual returns True when residual is set."""
        cv = CompressedVector(
            indices=np.array([0], dtype=np.uint8),
            residual=b"\x01",
            config_hash="abc",
            dimension=1,
            bit_width=4,
        )
        assert cv.has_residual is True

    def test_has_residual_false(self) -> None:
        """has_residual returns False when residual is None."""
        cv = CompressedVector(
            indices=np.array([0], dtype=np.uint8),
            residual=None,
            config_hash="abc",
            dimension=1,
            bit_width=4,
        )
        assert cv.has_residual is False

    def test_size_bytes_positive(self) -> None:
        """size_bytes returns a positive integer."""
        cv = CompressedVector(
            indices=np.array([0, 1, 2, 3], dtype=np.uint8),
            residual=None,
            config_hash="abc123",
            dimension=4,
            bit_width=4,
        )
        assert cv.size_bytes > 0

    def test_size_bytes_larger_with_residual(self) -> None:
        """Residual increases the byte count."""
        indices = np.array([0, 1, 2, 3], dtype=np.uint8)
        cv_no_res = CompressedVector(
            indices=indices,
            residual=None,
            config_hash="abc123",
            dimension=4,
            bit_width=4,
        )
        cv_with_res = CompressedVector(
            indices=indices,
            residual=b"\x00" * 100,
            config_hash="abc123",
            dimension=4,
            bit_width=4,
        )
        assert cv_with_res.size_bytes > cv_no_res.size_bytes


# ---------------------------------------------------------------------------
# Serialization
# ---------------------------------------------------------------------------


class TestSerialization:
    """Tests for to_bytes / from_bytes serialization."""

    def test_to_bytes_produces_bytes(self) -> None:
        """to_bytes returns a non-empty bytes object."""
        cv = CompressedVector(
            indices=np.array([3, 7, 1, 15], dtype=np.uint8),
            residual=None,
            config_hash="abc123",
            dimension=4,
            bit_width=4,
        )
        result = cv.to_bytes()
        assert isinstance(result, bytes)
        assert len(result) > 0

    def test_serialization_includes_config_hash(self) -> None:
        """Config hash is present in serialized bytes."""
        cv = CompressedVector(
            indices=np.array([0, 1], dtype=np.uint8),
            residual=None,
            config_hash="myhash",
            dimension=2,
            bit_width=4,
        )
        raw = cv.to_bytes()
        # Config hash occupies bytes 1..65 (after version byte)
        hash_region = raw[1:65]
        assert b"myhash" in hash_region

    def test_from_bytes_round_trip(self) -> None:
        """from_bytes(to_bytes(cv)) reproduces all fields."""
        cv = CompressedVector(
            indices=np.array([3, 7, 1, 15], dtype=np.uint8),
            residual=None,
            config_hash="abc123",
            dimension=4,
            bit_width=4,
        )
        restored = CompressedVector.from_bytes(cv.to_bytes())
        assert restored.dimension == cv.dimension
        assert restored.bit_width == cv.bit_width
        assert restored.config_hash == cv.config_hash
        assert np.array_equal(restored.indices, cv.indices)
        assert restored.residual == cv.residual

    def test_from_bytes_corrupted_data_raises(self) -> None:
        """Truncated data raises ValueError."""
        with pytest.raises(ValueError):
            CompressedVector.from_bytes(b"\x00\x01\x02\x03\x04")


# ---------------------------------------------------------------------------
# Immutability
# ---------------------------------------------------------------------------


class TestImmutability:
    """Tests for frozen dataclass immutability."""

    def test_frozen_rejects_field_assignment(self) -> None:
        """Assigning to any field raises FrozenInstanceError."""
        cv = CompressedVector(
            indices=np.array([0, 1], dtype=np.uint8),
            residual=None,
            config_hash="abc",
            dimension=2,
            bit_width=4,
        )
        with pytest.raises(dataclasses.FrozenInstanceError):
            cv.dimension = 99  # type: ignore[misc]
