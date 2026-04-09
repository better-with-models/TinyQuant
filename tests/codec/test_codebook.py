"""Unit tests for Codebook value object."""

from __future__ import annotations

import numpy as np
import pytest
from numpy.typing import NDArray

from tinyquant_cpu.codec.codebook import Codebook
from tinyquant_cpu.codec.codec_config import CodecConfig

# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------


class TestTraining:
    """Tests for Codebook.train factory."""

    def test_train_produces_correct_entry_count_4bit(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """4-bit codebook has 16 entries."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        assert codebook.num_entries == 16

    def test_train_produces_correct_entry_count_2bit(
        self, config_2bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """2-bit codebook has 4 entries."""
        codebook = Codebook.train(sample_vectors, config_2bit)
        assert codebook.num_entries == 4

    def test_train_entries_are_sorted(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """Entries are in ascending order."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        assert np.all(np.diff(codebook.entries) > 0)

    def test_train_entries_have_no_duplicates(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """All entries are distinct."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        assert len(np.unique(codebook.entries)) == codebook.num_entries

    def test_train_with_too_few_vectors_raises(self, config_4bit: CodecConfig) -> None:
        """Insufficient training data raises ValueError."""
        constant = np.zeros((1, 64), dtype=np.float32)
        with pytest.raises(ValueError, match=r"[Ii]nsufficient|unique"):
            Codebook.train(constant, config_4bit)

    def test_train_is_deterministic(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """Same training data + config produces identical codebook."""
        cb1 = Codebook.train(sample_vectors, config_4bit)
        cb2 = Codebook.train(sample_vectors, config_4bit)
        np.testing.assert_array_equal(cb1.entries, cb2.entries)


# ---------------------------------------------------------------------------
# Quantization
# ---------------------------------------------------------------------------


class TestQuantization:
    """Tests for Codebook.quantize method."""

    def test_quantize_returns_valid_indices(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """All indices are in [0, num_entries)."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        values = sample_vectors[0]
        indices = codebook.quantize(values)
        assert np.all(indices >= 0)
        assert np.all(indices < codebook.num_entries)

    def test_quantize_maps_to_nearest_entry(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """Each value maps to its closest codebook entry."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        values = sample_vectors[0]
        indices = codebook.quantize(values)
        for i, val in enumerate(values):
            # The chosen entry should be the closest one.
            distances = np.abs(codebook.entries - val)
            expected_idx = np.argmin(distances)
            assert indices[i] == expected_idx

    def test_quantize_exact_entry_value(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """A value equal to a codebook entry maps to that entry's index."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        exact_values = codebook.entries.copy()
        indices = codebook.quantize(exact_values)
        expected = np.arange(codebook.num_entries, dtype=np.uint8)
        np.testing.assert_array_equal(indices, expected)

    def test_quantize_output_dtype(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """Quantize returns uint8 array."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        indices = codebook.quantize(sample_vectors[0])
        assert indices.dtype == np.uint8


# ---------------------------------------------------------------------------
# Dequantization
# ---------------------------------------------------------------------------


class TestDequantization:
    """Tests for Codebook.dequantize method."""

    def test_dequantize_maps_indices_to_entries(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """Index i maps to entries[i]."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        indices = np.arange(codebook.num_entries, dtype=np.uint8)
        result = codebook.dequantize(indices)
        np.testing.assert_array_equal(result, codebook.entries)

    def test_dequantize_output_dtype(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """Dequantize returns float32 array."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        indices = np.array([0, 1, 2], dtype=np.uint8)
        result = codebook.dequantize(indices)
        assert result.dtype == np.float32

    def test_dequantize_invalid_index_raises(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """Index >= num_entries raises ValueError."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        bad_indices = np.array([0, 16], dtype=np.uint8)  # 16 is out of range for 4-bit
        with pytest.raises(ValueError, match=r"[Ii]ndex|range"):
            codebook.dequantize(bad_indices)


# ---------------------------------------------------------------------------
# Immutability
# ---------------------------------------------------------------------------


class TestImmutability:
    """Tests for frozen dataclass immutability."""

    def test_frozen_rejects_entry_modification(
        self, config_4bit: CodecConfig, sample_vectors: NDArray[np.float32]
    ) -> None:
        """Modifying entries array raises ValueError (non-writeable)."""
        codebook = Codebook.train(sample_vectors, config_4bit)
        with pytest.raises(ValueError, match=r"read-only|not writeable"):
            codebook.entries[0] = 999.0
