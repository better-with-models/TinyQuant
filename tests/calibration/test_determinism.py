"""Stub calibration tests for determinism — placeholder for Phase 10."""

import numpy as np

from tinyquant.codec import Codec, CodecConfig


def test_compression_is_deterministic() -> None:
    """Same input and config produce byte-identical output."""
    dim = 64
    config = CodecConfig(bit_width=4, dimension=dim, seed=99)
    codec = Codec()
    data = np.random.default_rng(99).standard_normal((20, dim)).astype(np.float32)
    codebook = codec.build_codebook(data, config)

    vector = data[0]
    result_a = codec.compress(vector, config, codebook)
    result_b = codec.compress(vector, config, codebook)

    assert result_a.to_bytes() == result_b.to_bytes()
