"""Shared pytest fixtures for TinyQuant tests."""

from __future__ import annotations

import numpy as np
import pytest
from numpy.typing import NDArray

from tinyquant.codec.codebook import Codebook
from tinyquant.codec.codec_config import CodecConfig
from tinyquant.codec.rotation_matrix import RotationMatrix


@pytest.fixture()
def config_4bit() -> CodecConfig:
    """Standard 4-bit codec config for tests."""
    return CodecConfig(bit_width=4, seed=42, dimension=64)


@pytest.fixture()
def config_2bit() -> CodecConfig:
    """Standard 2-bit codec config for tests."""
    return CodecConfig(bit_width=2, seed=42, dimension=64)


@pytest.fixture()
def config_8bit() -> CodecConfig:
    """Standard 8-bit codec config for tests."""
    return CodecConfig(bit_width=8, seed=42, dimension=64)


@pytest.fixture()
def sample_vectors() -> NDArray[np.float32]:
    """100 random vectors of dimension 64 for training/testing."""
    rng = np.random.default_rng(42)
    return rng.standard_normal((100, 64)).astype(np.float32)


@pytest.fixture()
def sample_vector() -> NDArray[np.float32]:
    """Single random vector of dimension 64."""
    rng = np.random.default_rng(42)
    return rng.standard_normal(64).astype(np.float32)


@pytest.fixture()
def trained_codebook(
    config_4bit: CodecConfig,
    sample_vectors: NDArray[np.float32],
) -> Codebook:
    """Codebook trained on sample vectors with 4-bit config."""
    return Codebook.train(sample_vectors, config_4bit)


@pytest.fixture()
def rotation_matrix(config_4bit: CodecConfig) -> RotationMatrix:
    """Rotation matrix generated from 4-bit config."""
    return RotationMatrix.from_config(config_4bit)
