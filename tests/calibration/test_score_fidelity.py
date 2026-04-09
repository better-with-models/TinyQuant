"""Calibration tests for score fidelity — VAL-01.

Validates that compressed-then-decompressed vectors preserve cosine
similarity rankings relative to the original FP32 vectors.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

from tinyquant.codec.codebook import Codebook
from tinyquant.codec.codec import Codec
from tinyquant.codec.codec_config import CodecConfig

if TYPE_CHECKING:
    from numpy.typing import NDArray


# ------------------------------------------------------------------
# Helpers
# ------------------------------------------------------------------


def _cosine_similarities(
    corpus: NDArray[np.float32],
    query: NDArray[np.float32],
) -> NDArray[np.float32]:
    """Cosine similarity of *query* against every row in *corpus*."""
    norms = np.linalg.norm(corpus, axis=1) * np.linalg.norm(query)
    norms = np.maximum(norms, 1e-12)
    result: NDArray[np.float32] = (corpus @ query / norms).astype(np.float32)
    return result


def _decompress_corpus(
    corpus: NDArray[np.float32],
    config: CodecConfig,
    codebook: Codebook,
    codec: Codec,
) -> NDArray[np.float32]:
    """Compress then decompress every vector in *corpus*."""
    compressed = codec.compress_batch(corpus, config, codebook)
    return codec.decompress_batch(compressed, config, codebook)


def _compute_pearson_rho(
    corpus: NDArray[np.float32],
    decompressed: NDArray[np.float32],
    queries: NDArray[np.float32],
    num_queries: int = 20,
) -> float:
    """Mean Pearson rho over *num_queries* queries."""
    rhos: list[float] = []
    for q in queries[:num_queries]:
        orig_sims = _cosine_similarities(corpus, q)
        approx_sims = _cosine_similarities(decompressed, q)
        rho = float(np.corrcoef(orig_sims, approx_sims)[0, 1])
        rhos.append(rho)
    return float(np.mean(rhos))


# ------------------------------------------------------------------
# Tests
# ------------------------------------------------------------------


def test_pearson_rho_4bit_with_residuals(
    gold_corpus_10k: NDArray[np.float32],
    query_vectors_100: NDArray[np.float32],
    calibrated_config: CodecConfig,
    trained_codebook_10k: Codebook,
    codec: Codec,
) -> None:
    """4-bit + residuals: Pearson rho >= 0.995."""
    decompressed = _decompress_corpus(
        gold_corpus_10k,
        calibrated_config,
        trained_codebook_10k,
        codec,
    )
    rho = _compute_pearson_rho(gold_corpus_10k, decompressed, query_vectors_100)
    assert rho >= 0.995, f"Pearson rho {rho:.6f} below 0.995"


def test_pearson_rho_4bit_without_residuals(
    gold_corpus_10k: NDArray[np.float32],
    query_vectors_100: NDArray[np.float32],
    calibrated_config_no_residual: CodecConfig,
    trained_codebook_10k_no_residual: Codebook,
    codec: Codec,
) -> None:
    """4-bit without residuals: Pearson rho >= 0.97."""
    decompressed = _decompress_corpus(
        gold_corpus_10k,
        calibrated_config_no_residual,
        trained_codebook_10k_no_residual,
        codec,
    )
    rho = _compute_pearson_rho(gold_corpus_10k, decompressed, query_vectors_100)
    assert rho >= 0.97, f"Pearson rho {rho:.6f} below 0.97"


def test_pearson_rho_2bit_with_residuals(
    gold_corpus_10k: NDArray[np.float32],
    query_vectors_100: NDArray[np.float32],
    calibrated_config_2bit: CodecConfig,
    trained_codebook_10k_2bit: Codebook,
    codec: Codec,
) -> None:
    """2-bit + residuals: Pearson rho >= 0.95."""
    decompressed = _decompress_corpus(
        gold_corpus_10k,
        calibrated_config_2bit,
        trained_codebook_10k_2bit,
        codec,
    )
    rho = _compute_pearson_rho(gold_corpus_10k, decompressed, query_vectors_100)
    assert rho >= 0.95, f"Pearson rho {rho:.6f} below 0.95"


def test_top10_overlap_4bit(
    gold_corpus_10k: NDArray[np.float32],
    query_vectors_100: NDArray[np.float32],
    calibrated_config: CodecConfig,
    trained_codebook_10k: Codebook,
    codec: Codec,
) -> None:
    """4-bit + residuals: top-10 neighbor overlap >= 80%."""
    decompressed = _decompress_corpus(
        gold_corpus_10k,
        calibrated_config,
        trained_codebook_10k,
        codec,
    )
    overlaps: list[float] = []
    for q in query_vectors_100[:20]:
        orig_sims = _cosine_similarities(gold_corpus_10k, q)
        approx_sims = _cosine_similarities(decompressed, q)
        top10_orig = set(np.argsort(orig_sims)[-10:])
        top10_approx = set(np.argsort(approx_sims)[-10:])
        overlaps.append(len(top10_orig & top10_approx) / 10.0)
    mean_overlap = float(np.mean(overlaps))
    assert mean_overlap >= 0.80, f"Top-10 overlap {mean_overlap:.2%} below 80%"


def test_residual_improves_rho(
    gold_corpus_10k: NDArray[np.float32],
    query_vectors_100: NDArray[np.float32],
    calibrated_config: CodecConfig,
    calibrated_config_no_residual: CodecConfig,
    trained_codebook_10k: Codebook,
    trained_codebook_10k_no_residual: Codebook,
    codec: Codec,
) -> None:
    """Residual correction strictly improves Pearson rho."""
    dec_with = _decompress_corpus(
        gold_corpus_10k,
        calibrated_config,
        trained_codebook_10k,
        codec,
    )
    dec_without = _decompress_corpus(
        gold_corpus_10k,
        calibrated_config_no_residual,
        trained_codebook_10k_no_residual,
        codec,
    )
    rho_with = _compute_pearson_rho(
        gold_corpus_10k,
        dec_with,
        query_vectors_100,
        num_queries=10,
    )
    rho_without = _compute_pearson_rho(
        gold_corpus_10k,
        dec_without,
        query_vectors_100,
        num_queries=10,
    )
    assert rho_with > rho_without, (
        f"Residual rho {rho_with:.6f} not better than no-residual {rho_without:.6f}"
    )


def test_fidelity_stable_across_corpus_sizes(
    gold_corpus_10k: NDArray[np.float32],
    query_vectors_100: NDArray[np.float32],
    calibrated_config: CodecConfig,
    codec: Codec,
) -> None:
    """Fidelity stable between 1K and 10K corpus (difference < 0.005)."""
    corpus_1k = gold_corpus_10k[:1_000]
    codebook_1k = Codebook.train(corpus_1k, calibrated_config)
    codebook_10k = Codebook.train(gold_corpus_10k, calibrated_config)

    dec_1k = _decompress_corpus(corpus_1k, calibrated_config, codebook_1k, codec)
    dec_10k = _decompress_corpus(
        gold_corpus_10k[:1_000],
        calibrated_config,
        codebook_10k,
        codec,
    )

    rho_1k = _compute_pearson_rho(corpus_1k, dec_1k, query_vectors_100, num_queries=10)
    rho_10k = _compute_pearson_rho(
        gold_corpus_10k[:1_000],
        dec_10k,
        query_vectors_100,
        num_queries=10,
    )

    diff = abs(rho_1k - rho_10k)
    assert diff < 0.005, (
        f"Fidelity difference {diff:.6f} exceeds 0.005 "
        f"(1K: {rho_1k:.6f}, 10K-trained: {rho_10k:.6f})"
    )
