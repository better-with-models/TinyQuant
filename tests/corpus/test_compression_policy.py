"""Tests for CompressionPolicy enum."""

from __future__ import annotations

import numpy as np

from tinyquant.corpus.compression_policy import CompressionPolicy


class TestRequiresCodec:
    """Tests for requires_codec property."""

    def test_compress_requires_codec(self) -> None:
        """COMPRESS requires the codec pipeline."""
        assert CompressionPolicy.COMPRESS.requires_codec() is True

    def test_passthrough_does_not_require_codec(self) -> None:
        """PASSTHROUGH bypasses the codec."""
        assert CompressionPolicy.PASSTHROUGH.requires_codec() is False

    def test_fp16_does_not_require_codec(self) -> None:
        """FP16 bypasses the codec."""
        assert CompressionPolicy.FP16.requires_codec() is False


class TestStorageDtype:
    """Tests for storage_dtype property."""

    def test_compress_storage_dtype(self) -> None:
        """COMPRESS stores as uint8 indices."""
        assert CompressionPolicy.COMPRESS.storage_dtype() == np.dtype(np.uint8)

    def test_passthrough_storage_dtype(self) -> None:
        """PASSTHROUGH stores as float32."""
        assert CompressionPolicy.PASSTHROUGH.storage_dtype() == np.dtype(np.float32)

    def test_fp16_storage_dtype(self) -> None:
        """FP16 stores as float16."""
        assert CompressionPolicy.FP16.storage_dtype() == np.dtype(np.float16)
