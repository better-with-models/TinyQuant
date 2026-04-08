"""Unit tests for CompressedVector value object."""

from __future__ import annotations

import dataclasses

import numpy as np
import pytest

from tinyquant.codec.compressed_vector import CompressedVector

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
            )

    def test_residual_none_when_disabled(self) -> None:
        """Residual field is None when correction was off."""
        cv = CompressedVector(
            indices=np.array([0, 1], dtype=np.uint8),
            residual=None,
            config_hash="abc123",
            dimension=2,
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
        )
        assert cv.residual == residual_data


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
        )
        assert cv.has_residual is True

    def test_has_residual_false(self) -> None:
        """has_residual returns False when residual is None."""
        cv = CompressedVector(
            indices=np.array([0], dtype=np.uint8),
            residual=None,
            config_hash="abc",
            dimension=1,
        )
        assert cv.has_residual is False

    def test_size_bytes_positive(self) -> None:
        """size_bytes returns a positive integer."""
        cv = CompressedVector(
            indices=np.array([0, 1, 2, 3], dtype=np.uint8),
            residual=None,
            config_hash="abc123",
            dimension=4,
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
        )
        cv_with_res = CompressedVector(
            indices=indices,
            residual=b"\x00" * 100,
            config_hash="abc123",
            dimension=4,
        )
        assert cv_with_res.size_bytes > cv_no_res.size_bytes


# ---------------------------------------------------------------------------
# Serialization stubs
# ---------------------------------------------------------------------------


class TestSerializationStubs:
    """Tests for Phase 6 serialization stubs."""

    def test_to_bytes_raises_not_implemented(self) -> None:
        """to_bytes raises NotImplementedError."""
        cv = CompressedVector(
            indices=np.array([0], dtype=np.uint8),
            residual=None,
            config_hash="abc",
            dimension=1,
        )
        with pytest.raises(NotImplementedError):
            cv.to_bytes()

    def test_from_bytes_raises_not_implemented(self) -> None:
        """from_bytes raises NotImplementedError."""
        with pytest.raises(NotImplementedError):
            CompressedVector.from_bytes(b"\x00")

    def test_serialization_stubs_have_informative_message(self) -> None:
        """Stub error messages mention Phase 6."""
        cv = CompressedVector(
            indices=np.array([0], dtype=np.uint8),
            residual=None,
            config_hash="abc",
            dimension=1,
        )
        with pytest.raises(NotImplementedError, match="Phase 6"):
            cv.to_bytes()


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
        )
        with pytest.raises(dataclasses.FrozenInstanceError):
            cv.dimension = 99  # type: ignore[misc]
