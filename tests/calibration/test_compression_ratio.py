"""Stub calibration tests for compression ratio — placeholder for Phase 10."""

import numpy as np

from tinyquant.codec import Codec, CodecConfig


def test_4bit_compression_ratio() -> None:
    """4-bit quantization achieves meaningful compression."""
    dim = 128
    config = CodecConfig(bit_width=4, dimension=dim, seed=0)
    codec = Codec()
    data = np.random.default_rng(0).standard_normal((10, dim)).astype(np.float32)
    codebook = codec.build_codebook(data, config)

    original_bytes = data[0].nbytes
    compressed = codec.compress(data[0], config, codebook)
    compressed_bytes = len(compressed.to_bytes())

    ratio = original_bytes / compressed_bytes
    assert ratio > 1.0, f"Compression ratio {ratio:.2f}x is not compressing"
