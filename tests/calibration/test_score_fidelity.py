"""Stub calibration tests for score fidelity — placeholder for Phase 10."""

import numpy as np

from tinyquant.codec import Codec, CodecConfig


def test_compressed_cosine_similarity_correlates_with_original() -> None:
    """Compressed vectors preserve relative similarity ordering."""
    rng = np.random.default_rng(42)
    dim = 64
    config = CodecConfig(bit_width=4, dimension=dim, seed=42)
    codec = Codec()
    data = rng.standard_normal((50, dim)).astype(np.float32)
    codebook = codec.build_codebook(data, config)

    query = rng.standard_normal(dim).astype(np.float32)
    original_sims = data @ query / (
        np.linalg.norm(data, axis=1) * np.linalg.norm(query)
    )

    decompressed = np.array(
        [
            codec.decompress(codec.compress(v, config, codebook), config, codebook)
            for v in data
        ]
    )
    approx_sims = decompressed @ query / (
        np.linalg.norm(decompressed, axis=1) * np.linalg.norm(query)
    )

    correlation = np.corrcoef(original_sims, approx_sims)[0, 1]
    assert correlation > 0.9, f"Pearson rho {correlation:.4f} below 0.9"
