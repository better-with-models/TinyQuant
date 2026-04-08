"""Unit tests for RotationMatrix value object."""

from __future__ import annotations

import numpy as np
import pytest
from hypothesis import given, settings
from hypothesis import strategies as st
from hypothesis.extra.numpy import arrays

from tinyquant.codec.codec_config import CodecConfig
from tinyquant.codec.rotation_matrix import RotationMatrix

# ---------------------------------------------------------------------------
# Factory construction
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for RotationMatrix.from_config factory."""

    def test_from_config_produces_correct_shape(self, config_4bit: CodecConfig) -> None:
        """Matrix shape is (dim, dim)."""
        rotation = RotationMatrix.from_config(config_4bit)
        assert rotation.matrix.shape == (64, 64)

    def test_from_config_is_deterministic(self, config_4bit: CodecConfig) -> None:
        """Same seed + dimension produces identical matrix."""
        r1 = RotationMatrix.from_config(config_4bit)
        r2 = RotationMatrix.from_config(config_4bit)
        np.testing.assert_array_equal(r1.matrix, r2.matrix)

    def test_different_seeds_produce_different_matrices(self) -> None:
        """Seed 42 != seed 99."""
        c1 = CodecConfig(bit_width=4, seed=42, dimension=64)
        c2 = CodecConfig(bit_width=4, seed=99, dimension=64)
        r1 = RotationMatrix.from_config(c1)
        r2 = RotationMatrix.from_config(c2)
        assert not np.array_equal(r1.matrix, r2.matrix)

    def test_different_dimensions_produce_different_matrices(self) -> None:
        """Dim 32 != dim 64."""
        c1 = CodecConfig(bit_width=4, seed=42, dimension=32)
        c2 = CodecConfig(bit_width=4, seed=42, dimension=64)
        r1 = RotationMatrix.from_config(c1)
        r2 = RotationMatrix.from_config(c2)
        assert r1.matrix.shape != r2.matrix.shape


# ---------------------------------------------------------------------------
# Orthogonality
# ---------------------------------------------------------------------------


class TestOrthogonality:
    """Tests verifying orthogonal matrix properties."""

    def test_matrix_is_orthogonal(self, config_4bit: CodecConfig) -> None:
        """R @ R.T approximately equals identity."""
        rotation = RotationMatrix.from_config(config_4bit)
        product = rotation.matrix @ rotation.matrix.T
        np.testing.assert_allclose(product, np.eye(64), atol=1e-10)

    def test_verify_orthogonality_returns_true(self, config_4bit: CodecConfig) -> None:
        """verify_orthogonality passes for a valid matrix."""
        rotation = RotationMatrix.from_config(config_4bit)
        assert rotation.verify_orthogonality()

    def test_corrupted_matrix_fails_orthogonality(
        self, config_4bit: CodecConfig
    ) -> None:
        """Manually perturbed matrix fails orthogonality check."""
        rotation = RotationMatrix.from_config(config_4bit)
        # Create a corrupted copy
        corrupted = rotation.matrix.copy()
        corrupted[0, 0] += 1.0
        bad_rotation = RotationMatrix(matrix=corrupted, seed=42, dimension=64)
        assert not bad_rotation.verify_orthogonality()


# ---------------------------------------------------------------------------
# Apply and inverse
# ---------------------------------------------------------------------------


class TestApplyAndInverse:
    """Tests for apply and apply_inverse methods."""

    def test_apply_changes_vector(
        self, config_4bit: CodecConfig, sample_vector: np.ndarray
    ) -> None:
        """apply(v) != v for non-trivial v."""
        rotation = RotationMatrix.from_config(config_4bit)
        rotated = rotation.apply(sample_vector)
        assert not np.allclose(rotated, sample_vector)

    def test_apply_inverse_recovers_original(
        self, config_4bit: CodecConfig, sample_vector: np.ndarray
    ) -> None:
        """apply_inverse(apply(v)) approximately equals v."""
        rotation = RotationMatrix.from_config(config_4bit)
        rotated = rotation.apply(sample_vector)
        recovered = rotation.apply_inverse(rotated)
        np.testing.assert_allclose(recovered, sample_vector, atol=1e-5)

    def test_apply_preserves_norm(
        self, config_4bit: CodecConfig, sample_vector: np.ndarray
    ) -> None:
        """Orthogonal transform preserves vector norm."""
        rotation = RotationMatrix.from_config(config_4bit)
        rotated = rotation.apply(sample_vector)
        np.testing.assert_allclose(
            np.linalg.norm(rotated),
            np.linalg.norm(sample_vector),
            rtol=1e-5,
        )

    def test_apply_dimension_mismatch_raises(self, config_4bit: CodecConfig) -> None:
        """Wrong-length vector raises ValueError."""
        rotation = RotationMatrix.from_config(config_4bit)
        wrong = np.ones(32, dtype=np.float32)
        with pytest.raises(ValueError, match="dimension"):
            rotation.apply(wrong)


# ---------------------------------------------------------------------------
# Property-based (hypothesis)
# ---------------------------------------------------------------------------


class TestHypothesis:
    """Property-based tests using hypothesis."""

    @settings(max_examples=20)
    @given(
        vector=arrays(
            dtype=np.float32,
            shape=(64,),
            elements=st.floats(
                min_value=-100,
                max_value=100,
                allow_nan=False,
                allow_infinity=False,
            ),
        ),
    )
    def test_round_trip_preserves_vector(self, vector: np.ndarray) -> None:
        """For any valid vector, apply_inverse(apply(v)) approximately equals v."""
        config = CodecConfig(bit_width=4, seed=42, dimension=64)
        rotation = RotationMatrix.from_config(config)
        recovered = rotation.apply_inverse(rotation.apply(vector))
        np.testing.assert_allclose(recovered, vector, atol=1e-4)
