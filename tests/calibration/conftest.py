"""Calibration test fixtures for Phase 10 validation suite.

Provides large-scale synthetic data at dimension 768 with fixed seeds
for deterministic CI. Module-scoped fixtures amortize expensive codebook
training and batch compression across tests.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np
import pytest

from tinyquant.codec.codebook import Codebook
from tinyquant.codec.codec import Codec
from tinyquant.codec.codec_config import CodecConfig

if TYPE_CHECKING:
    from numpy.typing import NDArray

_DIM = 768
_CORPUS_SIZE = 10_000
_QUERY_SIZE = 100
_SEED = 42


# ------------------------------------------------------------------
# Utility (not a fixture)
# ------------------------------------------------------------------


def cosine_similarities(
    corpus: NDArray[np.float32],
    query: NDArray[np.float32],
) -> NDArray[np.float32]:
    """Compute cosine similarity of *query* against every row in *corpus*."""
    norms = np.linalg.norm(corpus, axis=1) * np.linalg.norm(query)
    norms = np.maximum(norms, 1e-12)
    result: NDArray[np.float32] = (corpus @ query / norms).astype(np.float32)
    return result


# ------------------------------------------------------------------
# Config fixtures
# ------------------------------------------------------------------


@pytest.fixture(scope="module")
def calibrated_config() -> CodecConfig:
    """4-bit, residual-enabled config at dim 768."""
    return CodecConfig(bit_width=4, dimension=_DIM, seed=_SEED, residual_enabled=True)


@pytest.fixture(scope="module")
def calibrated_config_no_residual() -> CodecConfig:
    """4-bit, residual-disabled config at dim 768."""
    return CodecConfig(bit_width=4, dimension=_DIM, seed=_SEED, residual_enabled=False)


@pytest.fixture(scope="module")
def calibrated_config_2bit() -> CodecConfig:
    """2-bit, residual-enabled config at dim 768."""
    return CodecConfig(bit_width=2, dimension=_DIM, seed=_SEED, residual_enabled=True)


# ------------------------------------------------------------------
# Data fixtures
# ------------------------------------------------------------------


@pytest.fixture(scope="module")
def gold_corpus_10k() -> NDArray[np.float32]:
    """10,000 synthetic FP32 vectors at dim 768 (seed 42)."""
    rng = np.random.default_rng(_SEED)
    return rng.standard_normal((_CORPUS_SIZE, _DIM)).astype(np.float32)


@pytest.fixture(scope="module")
def query_vectors_100() -> NDArray[np.float32]:
    """100 query vectors at dim 768 from a separate seed."""
    rng = np.random.default_rng(_SEED + 1)
    return rng.standard_normal((_QUERY_SIZE, _DIM)).astype(np.float32)


# ------------------------------------------------------------------
# Codec / codebook fixtures
# ------------------------------------------------------------------


@pytest.fixture(scope="module")
def codec() -> Codec:
    """Shared Codec instance."""
    return Codec()


@pytest.fixture(scope="module")
def trained_codebook_10k(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config: CodecConfig,
) -> Codebook:
    """Codebook trained on the full 10K gold corpus with 4-bit config."""
    return Codebook.train(gold_corpus_10k, calibrated_config)


@pytest.fixture(scope="module")
def trained_codebook_10k_no_residual(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config_no_residual: CodecConfig,
) -> Codebook:
    """Codebook trained with 4-bit no-residual config."""
    return Codebook.train(gold_corpus_10k, calibrated_config_no_residual)


@pytest.fixture(scope="module")
def trained_codebook_10k_2bit(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config_2bit: CodecConfig,
) -> Codebook:
    """Codebook trained with 2-bit config."""
    return Codebook.train(gold_corpus_10k, calibrated_config_2bit)
