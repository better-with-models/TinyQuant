"""Tests for the Codec domain service."""

from __future__ import annotations

import numpy as np
import pytest
from hypothesis import given, settings
from hypothesis import strategies as st
from numpy.typing import NDArray

from tinyquant.codec._errors import (
    CodebookIncompatibleError,
    ConfigMismatchError,
    DimensionMismatchError,
)
from tinyquant.codec._quantize import scalar_dequantize, scalar_quantize
from tinyquant.codec.codebook import Codebook
from tinyquant.codec.codec import Codec, compress, decompress
from tinyquant.codec.codec_config import CodecConfig
from tinyquant.codec.compressed_vector import CompressedVector

# ---------------------------------------------------------------------------
# Fixtures local to this module
# ---------------------------------------------------------------------------


@pytest.fixture()
def codec() -> Codec:
    """Fresh stateless Codec instance."""
    return Codec()


@pytest.fixture()
def config_no_residual() -> CodecConfig:
    """4-bit config with residual disabled."""
    return CodecConfig(bit_width=4, seed=42, dimension=64, residual_enabled=False)


@pytest.fixture()
def codebook_no_residual(
    config_no_residual: CodecConfig,
    sample_vectors: NDArray[np.float32],
) -> Codebook:
    """Codebook trained under no-residual config."""
    return Codebook.train(sample_vectors, config_no_residual)


# ===========================================================================
# compress() tests
# ===========================================================================


class TestCompress:
    """Tests for Codec.compress."""

    def test_compress_returns_compressed_vector(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Result is a CompressedVector instance."""
        result = codec.compress(sample_vector, config_4bit, trained_codebook)
        assert isinstance(result, CompressedVector)

    def test_compress_indices_in_valid_range(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """All indices are in [0, 2^bit_width)."""
        result = codec.compress(sample_vector, config_4bit, trained_codebook)
        assert np.all(result.indices < 2**config_4bit.bit_width)

    def test_compress_config_hash_matches(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Result config_hash matches the input config."""
        result = codec.compress(sample_vector, config_4bit, trained_codebook)
        assert result.config_hash == config_4bit.config_hash

    def test_compress_dimension_match(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Result dimension matches config.dimension."""
        result = codec.compress(sample_vector, config_4bit, trained_codebook)
        assert result.dimension == config_4bit.dimension

    def test_compress_deterministic(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Same inputs always produce identical output."""
        a = codec.compress(sample_vector, config_4bit, trained_codebook)
        b = codec.compress(sample_vector, config_4bit, trained_codebook)
        assert np.array_equal(a.indices, b.indices)
        assert a.residual == b.residual
        assert a.config_hash == b.config_hash

    def test_compress_dimension_mismatch_raises(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
    ) -> None:
        """Wrong-length vector raises DimensionMismatchError."""
        wrong = np.zeros(32, dtype=np.float32)
        with pytest.raises(DimensionMismatchError):
            codec.compress(wrong, config_4bit, trained_codebook)

    def test_compress_codebook_incompatible_raises(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        sample_vector: NDArray[np.float32],
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Mismatched bit_width raises CodebookIncompatibleError."""
        config_2 = CodecConfig(bit_width=2, seed=42, dimension=64)
        codebook_2 = Codebook.train(sample_vectors, config_2)
        with pytest.raises(CodebookIncompatibleError):
            codec.compress(sample_vector, config_4bit, codebook_2)

    def test_compress_with_residual(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Residual enabled produces non-null residual."""
        result = codec.compress(sample_vector, config_4bit, trained_codebook)
        assert result.has_residual
        assert result.residual is not None
        assert len(result.residual) > 0

    def test_compress_without_residual(
        self,
        codec: Codec,
        config_no_residual: CodecConfig,
        codebook_no_residual: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Residual disabled produces null residual."""
        result = codec.compress(sample_vector, config_no_residual, codebook_no_residual)
        assert not result.has_residual
        assert result.residual is None


# ===========================================================================
# decompress() tests
# ===========================================================================


class TestDecompress:
    """Tests for Codec.decompress."""

    def test_decompress_returns_fp32_array(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Result dtype is float32."""
        compressed = codec.compress(sample_vector, config_4bit, trained_codebook)
        result = codec.decompress(compressed, config_4bit, trained_codebook)
        assert result.dtype == np.float32

    def test_decompress_correct_dimension(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Result length matches config.dimension."""
        compressed = codec.compress(sample_vector, config_4bit, trained_codebook)
        result = codec.decompress(compressed, config_4bit, trained_codebook)
        assert len(result) == config_4bit.dimension

    def test_decompress_deterministic(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Same inputs produce identical output."""
        compressed = codec.compress(sample_vector, config_4bit, trained_codebook)
        a = codec.decompress(compressed, config_4bit, trained_codebook)
        b = codec.decompress(compressed, config_4bit, trained_codebook)
        assert np.array_equal(a, b)

    def test_decompress_config_mismatch_raises(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Wrong config hash raises ConfigMismatchError."""
        compressed = codec.compress(sample_vector, config_4bit, trained_codebook)
        different_config = CodecConfig(bit_width=4, seed=99, dimension=64)
        with pytest.raises(ConfigMismatchError):
            codec.decompress(compressed, different_config, trained_codebook)

    def test_decompress_codebook_incompatible_raises(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Mismatched codebook bit_width raises CodebookIncompatibleError."""
        compressed = codec.compress(sample_vector, config_4bit, trained_codebook)
        config_2 = CodecConfig(bit_width=2, seed=42, dimension=64)
        codebook_2 = Codebook.train(sample_vectors, config_2)
        with pytest.raises(CodebookIncompatibleError):
            codec.decompress(compressed, config_4bit, codebook_2)


# ===========================================================================
# Round-trip tests
# ===========================================================================


class TestRoundTrip:
    """Tests for compress then decompress fidelity."""

    def test_round_trip_recovers_shape(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Output shape matches input."""
        compressed = codec.compress(sample_vector, config_4bit, trained_codebook)
        result = codec.decompress(compressed, config_4bit, trained_codebook)
        assert result.shape == sample_vector.shape

    def test_round_trip_error_is_bounded(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """MSE is below a reasonable threshold."""
        compressed = codec.compress(sample_vector, config_4bit, trained_codebook)
        result = codec.decompress(compressed, config_4bit, trained_codebook)
        mse = float(np.mean((sample_vector - result) ** 2))
        assert mse < 1.0

    def test_round_trip_with_residual_has_lower_error(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        config_no_residual: CodecConfig,
        codebook_no_residual: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Residual correction reduces reconstruction error."""
        with_res = codec.compress(sample_vector, config_4bit, trained_codebook)
        result_with = codec.decompress(with_res, config_4bit, trained_codebook)
        mse_with = float(np.mean((sample_vector - result_with) ** 2))

        without_res = codec.compress(
            sample_vector, config_no_residual, codebook_no_residual
        )
        result_without = codec.decompress(
            without_res, config_no_residual, codebook_no_residual
        )
        mse_without = float(np.mean((sample_vector - result_without) ** 2))

        assert mse_with <= mse_without


# ===========================================================================
# Batch tests
# ===========================================================================


class TestBatch:
    """Tests for compress_batch / decompress_batch."""

    def test_compress_batch_count_matches(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """N inputs produce N outputs."""
        batch = sample_vectors[:10]
        results = codec.compress_batch(batch, config_4bit, trained_codebook)
        assert len(results) == 10

    def test_compress_batch_matches_individual(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Batch results identical to one-at-a-time."""
        batch = sample_vectors[:5]
        batch_results = codec.compress_batch(batch, config_4bit, trained_codebook)
        for i in range(5):
            individual = codec.compress(batch[i], config_4bit, trained_codebook)
            assert np.array_equal(batch_results[i].indices, individual.indices)
            assert batch_results[i].residual == individual.residual

    def test_decompress_batch_count_matches(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """N inputs produce N rows."""
        batch = sample_vectors[:10]
        compressed = codec.compress_batch(batch, config_4bit, trained_codebook)
        result = codec.decompress_batch(compressed, config_4bit, trained_codebook)
        assert result.shape[0] == 10

    def test_decompress_batch_matches_individual(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Batch decompression identical to one-at-a-time."""
        batch = sample_vectors[:5]
        compressed = codec.compress_batch(batch, config_4bit, trained_codebook)
        batch_result = codec.decompress_batch(compressed, config_4bit, trained_codebook)
        for i in range(5):
            individual = codec.decompress(compressed[i], config_4bit, trained_codebook)
            assert np.array_equal(batch_result[i], individual)


# ===========================================================================
# Build helper tests
# ===========================================================================


class TestBuildHelpers:
    """Tests for build_codebook and build_rotation."""

    def test_build_codebook_entry_count(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Returned codebook has 2^bit_width entries."""
        codebook = codec.build_codebook(sample_vectors, config_4bit)
        assert codebook.num_entries == 2**config_4bit.bit_width

    def test_build_codebook_too_few_vectors_raises(
        self,
        codec: Codec,
    ) -> None:
        """Constant training data raises ValueError."""
        constant = np.ones((10, 2), dtype=np.float32)
        config = CodecConfig(bit_width=8, seed=42, dimension=2)
        with pytest.raises(ValueError, match="Insufficient"):
            codec.build_codebook(constant, config)

    def test_build_rotation_is_orthogonal(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
    ) -> None:
        """Rotation passes orthogonality check."""
        rotation = codec.build_rotation(config_4bit)
        assert rotation.verify_orthogonality()

    def test_build_rotation_deterministic(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
    ) -> None:
        """Same config produces same rotation matrix."""
        a = codec.build_rotation(config_4bit)
        b = codec.build_rotation(config_4bit)
        assert np.array_equal(a.matrix, b.matrix)


# ===========================================================================
# Module-level function tests
# ===========================================================================


class TestModuleFunctions:
    """Tests for module-level compress() and decompress()."""

    def test_module_compress(
        self,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Module-level compress returns CompressedVector."""
        result = compress(sample_vector, config_4bit, trained_codebook)
        assert isinstance(result, CompressedVector)

    def test_module_decompress(
        self,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Module-level decompress returns correct FP32 array."""
        compressed = compress(sample_vector, config_4bit, trained_codebook)
        result = decompress(compressed, config_4bit, trained_codebook)
        assert result.dtype == np.float32
        assert len(result) == config_4bit.dimension


# ===========================================================================
# Property-based tests
# ===========================================================================


class TestQuantizeHelpers:
    """Tests for private _quantize standalone functions."""

    def test_scalar_quantize_nearest_neighbor(
        self,
        trained_codebook: Codebook,
    ) -> None:
        """scalar_quantize matches Codebook.quantize."""
        values = np.array([0.0, 0.5, -0.5, 1.0], dtype=np.float32)
        expected = trained_codebook.quantize(values)
        result = scalar_quantize(values, trained_codebook.entries)
        assert np.array_equal(result, expected)

    def test_scalar_dequantize_lookup(
        self,
        trained_codebook: Codebook,
    ) -> None:
        """scalar_dequantize matches Codebook.dequantize."""
        indices = np.array([0, 5, 15], dtype=np.uint8)
        expected = trained_codebook.dequantize(indices)
        result = scalar_dequantize(indices, trained_codebook.entries)
        assert np.array_equal(result, expected)


class TestHypothesis:
    """Hypothesis property-based tests for Codec."""

    @settings(max_examples=20, deadline=None)
    @given(
        seed=st.integers(min_value=0, max_value=1000),
    )
    def test_compress_decompress_determinism(self, seed: int) -> None:
        """Double compress and decompress always identical."""
        rng = np.random.default_rng(seed)
        dim = 16
        config = CodecConfig(bit_width=4, seed=42, dimension=dim)
        vectors = rng.standard_normal((50, dim)).astype(np.float32)
        vector = rng.standard_normal(dim).astype(np.float32)
        codebook = Codebook.train(vectors, config)
        codec = Codec()

        a = codec.compress(vector, config, codebook)
        b = codec.compress(vector, config, codebook)
        assert np.array_equal(a.indices, b.indices)
        assert a.residual == b.residual

        ra = codec.decompress(a, config, codebook)
        rb = codec.decompress(b, config, codebook)
        assert np.array_equal(ra, rb)

    @settings(max_examples=10, deadline=None)
    @given(
        n=st.integers(min_value=1, max_value=8),
        seed=st.integers(min_value=0, max_value=1000),
    )
    def test_batch_matches_individual(self, n: int, seed: int) -> None:
        """Batch and loop produce identical results."""
        rng = np.random.default_rng(seed)
        dim = 16
        config = CodecConfig(bit_width=4, seed=42, dimension=dim)
        training = rng.standard_normal((50, dim)).astype(np.float32)
        codebook = Codebook.train(training, config)
        codec = Codec()

        batch = rng.standard_normal((n, dim)).astype(np.float32)
        batch_results = codec.compress_batch(batch, config, codebook)

        for i in range(n):
            individual = codec.compress(batch[i], config, codebook)
            assert np.array_equal(batch_results[i].indices, individual.indices)
            assert batch_results[i].residual == individual.residual
