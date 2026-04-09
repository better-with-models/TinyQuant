"""Calibration tests for research alignment — VAL-04.

Validates that the implementation aligns with the research baseline
claims: random preconditioning eliminates normalization, two-stage
quantization preserves inner products, and 4-bit is the practical
default meeting both fidelity and compression targets.
"""

from __future__ import annotations

import math
from typing import TYPE_CHECKING

import numpy as np

from tinyquant_cpu.codec.codec import Codec
from tinyquant_cpu.codec.codec_config import CodecConfig
from tinyquant_cpu.codec.compressed_vector import _HEADER_SIZE

if TYPE_CHECKING:
    from numpy.typing import NDArray

    from tinyquant_cpu.codec.codebook import Codebook


def _cosine_similarities(
    corpus: NDArray[np.float32],
    query: NDArray[np.float32],
) -> NDArray[np.float32]:
    """Cosine similarity of *query* against every row in *corpus*."""
    norms = np.linalg.norm(corpus, axis=1) * np.linalg.norm(query)
    norms = np.maximum(norms, 1e-12)
    result: NDArray[np.float32] = (corpus @ query / norms).astype(np.float32)
    return result


def test_random_preconditioning_eliminates_normalization(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config_no_residual: CodecConfig,
    trained_codebook_10k_no_residual: Codebook,
    codec: Codec,
) -> None:
    """CompressedVector has no per-block scales or offsets.

    The PolarQuant research claim is that random orthogonal
    preconditioning makes per-block normalization unnecessary.
    We verify this by checking that the serialized size matches
    exactly: header + packed indices + residual flag.
    """
    vector = gold_corpus_10k[0]
    compressed = codec.compress(
        vector,
        calibrated_config_no_residual,
        trained_codebook_10k_no_residual,
    )

    dim = calibrated_config_no_residual.dimension
    bit_width = calibrated_config_no_residual.bit_width
    packed_len = math.ceil(dim * bit_width / 8)
    expected_len = _HEADER_SIZE + packed_len + 1  # +1 for residual flag

    raw = compressed.to_bytes()
    assert len(raw) == expected_len, (
        f"Extra bytes ({len(raw) - expected_len}) detected — "
        f"possible per-block normalization overhead"
    )
    assert compressed.residual is None


def test_two_stage_preserves_inner_product(
    gold_corpus_10k: NDArray[np.float32],
    query_vectors_100: NDArray[np.float32],
    calibrated_config: CodecConfig,
    calibrated_config_no_residual: CodecConfig,
    trained_codebook_10k: Codebook,
    trained_codebook_10k_no_residual: Codebook,
    codec: Codec,
) -> None:
    """Two-stage (residual) quantization improves inner-product fidelity.

    The QJL research baseline predicts that the residual stage
    measurably improves Pearson rho for cosine similarity scores.
    """
    subset = gold_corpus_10k[:1_000]
    queries = query_vectors_100[:10]

    dec_with_res = codec.decompress_batch(
        codec.compress_batch(subset, calibrated_config, trained_codebook_10k),
        calibrated_config,
        trained_codebook_10k,
    )
    dec_without_res = codec.decompress_batch(
        codec.compress_batch(
            subset,
            calibrated_config_no_residual,
            trained_codebook_10k_no_residual,
        ),
        calibrated_config_no_residual,
        trained_codebook_10k_no_residual,
    )

    rhos_with: list[float] = []
    rhos_without: list[float] = []
    for q in queries:
        orig = _cosine_similarities(subset, q)
        sims_w = _cosine_similarities(dec_with_res, q)
        sims_wo = _cosine_similarities(dec_without_res, q)
        rhos_with.append(float(np.corrcoef(orig, sims_w)[0, 1]))
        rhos_without.append(float(np.corrcoef(orig, sims_wo)[0, 1]))

    mean_with = float(np.mean(rhos_with))
    mean_without = float(np.mean(rhos_without))
    assert mean_with > mean_without, (
        f"Residual rho {mean_with:.6f} not better than stage-1-only {mean_without:.6f}"
    )


def test_4bit_is_practical_default(
    gold_corpus_10k: NDArray[np.float32],
    query_vectors_100: NDArray[np.float32],
    calibrated_config: CodecConfig,
    trained_codebook_10k: Codebook,
    codec: Codec,
) -> None:
    """4-bit is the practical sweet spot: high fidelity + meaningful compression.

    With residuals (FP16): fidelity rho >= 0.995, ratio ~1.6x.
    Without residuals: ratio ~8x (but lower fidelity).
    This test validates that 4-bit with residuals meets the fidelity
    target and still achieves better-than-unity compression.
    """
    # Fidelity check on a subset for speed
    subset = gold_corpus_10k[:1_000]
    dec = codec.decompress_batch(
        codec.compress_batch(subset, calibrated_config, trained_codebook_10k),
        calibrated_config,
        trained_codebook_10k,
    )

    rhos: list[float] = []
    for q in query_vectors_100[:10]:
        orig = _cosine_similarities(subset, q)
        approx = _cosine_similarities(dec, q)
        rhos.append(float(np.corrcoef(orig, approx)[0, 1]))
    mean_rho = float(np.mean(rhos))
    assert mean_rho >= 0.995, f"Fidelity rho {mean_rho:.6f} below 0.995"

    # Compression check: residuals add FP16 overhead, so ratio is ~1.6x
    vector = gold_corpus_10k[0]
    compressed = codec.compress(vector, calibrated_config, trained_codebook_10k)
    ratio = vector.nbytes / compressed.size_bytes
    assert ratio >= 1.5, f"Compression ratio {ratio:.2f}x below 1.5x"
