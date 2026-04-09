"""Calibration tests for compression ratio — VAL-02.

Validates that the serialized format achieves the storage reduction
claimed by the research baseline (~8x at 4-bit).
"""

from __future__ import annotations

import math
import struct
from typing import TYPE_CHECKING

import numpy as np

from tinyquant_cpu.codec.codec import Codec
from tinyquant_cpu.codec.codec_config import CodecConfig
from tinyquant_cpu.codec.compressed_vector import (
    _HEADER_FORMAT,
    _HEADER_SIZE,
)

if TYPE_CHECKING:
    from numpy.typing import NDArray

    from tinyquant_cpu.codec.codebook import Codebook


def test_4bit_compression_ratio(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config_no_residual: CodecConfig,
    trained_codebook_10k_no_residual: Codebook,
    codec: Codec,
) -> None:
    """4-bit quantization achieves >= 7x payload compression."""
    vector = gold_corpus_10k[0]
    original_bytes = vector.nbytes  # 768 * 4 = 3072

    compressed = codec.compress(
        vector,
        calibrated_config_no_residual,
        trained_codebook_10k_no_residual,
    )
    # size_bytes is payload only: packed indices + residual
    ratio = original_bytes / compressed.size_bytes
    assert ratio >= 7.0, f"Compression ratio {ratio:.2f}x below 7x"


def test_4bit_with_residuals_compression_ratio(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config: CodecConfig,
    trained_codebook_10k: Codebook,
    codec: Codec,
) -> None:
    """4-bit + FP16 residuals achieves >= 1.5x payload compression.

    The residual stores FP16 diffs (2 bytes/dim), so payload is
    packed_indices + residual = dim*4/8 + dim*2 bytes.  At dim 768
    that is 384 + 1536 = 1920 vs 3072 original → ~1.6x.
    """
    vector = gold_corpus_10k[0]
    original_bytes = vector.nbytes

    compressed = codec.compress(vector, calibrated_config, trained_codebook_10k)
    ratio = original_bytes / compressed.size_bytes
    assert ratio >= 1.5, (
        f"Compression ratio {ratio:.2f}x below 1.5x "
        f"(payload={compressed.size_bytes} bytes)"
    )


def test_no_normalization_overhead(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config_no_residual: CodecConfig,
    trained_codebook_10k_no_residual: Codebook,
    codec: Codec,
) -> None:
    """CompressedVector stores no per-block scales or normalization metadata.

    The serialized format is exactly: header + packed indices + residual flag
    (+ optional residual data). No extra bytes should appear.
    """
    vector = gold_corpus_10k[0]
    compressed = codec.compress(
        vector,
        calibrated_config_no_residual,
        trained_codebook_10k_no_residual,
    )
    raw = compressed.to_bytes()

    dim = calibrated_config_no_residual.dimension
    bit_width = calibrated_config_no_residual.bit_width
    packed_len = math.ceil(dim * bit_width / 8)

    # Expected: header + packed indices + 1 byte residual flag (0x00)
    expected_len = _HEADER_SIZE + packed_len + 1
    assert len(raw) == expected_len, (
        f"Serialized length {len(raw)} != expected {expected_len}; "
        f"extra bytes suggest normalization overhead"
    )

    # Verify header can be parsed and residual flag is 0x00
    _version, _hash_raw, out_dim, out_bw = struct.unpack_from(_HEADER_FORMAT, raw)
    assert out_dim == dim
    assert out_bw == bit_width
    assert raw[_HEADER_SIZE + packed_len] == 0x00, "Residual flag should be 0x00"
