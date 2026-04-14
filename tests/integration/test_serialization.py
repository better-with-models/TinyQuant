"""Integration tests for CompressedVector serialization (EB-01)."""

from __future__ import annotations

import numpy as np
import pytest
from numpy.typing import NDArray

from tinyquant_py_reference.codec.codebook import Codebook
from tinyquant_py_reference.codec.codec import Codec
from tinyquant_py_reference.codec.codec_config import CodecConfig
from tinyquant_py_reference.codec.compressed_vector import CompressedVector


@pytest.fixture()
def codec() -> Codec:
    """Shared codec instance."""
    return Codec()


@pytest.fixture()
def config_3072() -> CodecConfig:
    """Large-dimension config for stress testing."""
    return CodecConfig(bit_width=4, seed=42, dimension=3072)


@pytest.fixture()
def vectors_3072() -> NDArray[np.float32]:
    """Training vectors for dim=3072."""
    rng = np.random.default_rng(42)
    return rng.standard_normal((50, 3072)).astype(np.float32)


@pytest.fixture()
def codebook_3072(
    config_3072: CodecConfig,
    vectors_3072: NDArray[np.float32],
) -> Codebook:
    """Codebook trained for dim=3072."""
    return Codebook.train(vectors_3072, config_3072)


def _assert_cv_equal(a: CompressedVector, b: CompressedVector) -> None:
    """Field-by-field equality check for CompressedVector."""
    assert a.dimension == b.dimension
    assert a.bit_width == b.bit_width
    assert a.config_hash == b.config_hash
    assert np.array_equal(a.indices, b.indices)
    assert a.residual == b.residual


class TestSerializeDeserialize:
    """EB-01: CompressedVector serialization integration tests."""

    def test_serialize_deserialize_round_trip(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """from_bytes(to_bytes(cv)) produces identical CompressedVector."""
        cv = codec.compress(sample_vector, config_4bit, trained_codebook)
        restored = CompressedVector.from_bytes(cv.to_bytes())
        _assert_cv_equal(cv, restored)

    def test_serialization_format_version(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Version byte is present and correct."""
        cv = codec.compress(sample_vector, config_4bit, trained_codebook)
        raw = cv.to_bytes()
        assert raw[0] == 0x01

    def test_deserialization_rejects_wrong_version(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Unknown version byte raises ValueError."""
        cv = codec.compress(sample_vector, config_4bit, trained_codebook)
        raw = bytearray(cv.to_bytes())
        raw[0] = 0xFF
        with pytest.raises(ValueError, match="unknown format version"):
            CompressedVector.from_bytes(bytes(raw))

    def test_deserialization_rejects_truncated_data(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Incomplete bytes raise ValueError."""
        cv = codec.compress(sample_vector, config_4bit, trained_codebook)
        raw = cv.to_bytes()
        with pytest.raises(ValueError):
            CompressedVector.from_bytes(raw[:10])

    def test_deserialization_rejects_corrupted_hash(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Tampered config hash is detectable after round trip."""
        cv = codec.compress(sample_vector, config_4bit, trained_codebook)
        raw = bytearray(cv.to_bytes())
        # Flip a byte in the hash region (bytes 1..33)
        raw[5] ^= 0xFF
        restored = CompressedVector.from_bytes(bytes(raw))
        assert restored.config_hash != cv.config_hash

    def test_large_dimension_serialization(
        self,
        codec: Codec,
        config_3072: CodecConfig,
        codebook_3072: Codebook,
        vectors_3072: NDArray[np.float32],
    ) -> None:
        """dim=3072 round-trips correctly."""
        cv = codec.compress(vectors_3072[0], config_3072, codebook_3072)
        restored = CompressedVector.from_bytes(cv.to_bytes())
        _assert_cv_equal(cv, restored)

    def test_residual_survives_serialization(
        self,
        codec: Codec,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Residual data preserved through bytes round trip."""
        config = CodecConfig(
            bit_width=4,
            seed=42,
            dimension=64,
            residual_enabled=True,
        )
        rng = np.random.default_rng(99)
        training = rng.standard_normal((100, 64)).astype(np.float32)
        codebook = Codebook.train(training, config)

        cv = codec.compress(sample_vector, config, codebook)
        assert cv.has_residual

        restored = CompressedVector.from_bytes(cv.to_bytes())
        assert restored.has_residual
        assert restored.residual == cv.residual
        _assert_cv_equal(cv, restored)
