"""Calibration tests for determinism — VAL-03.

Validates that the same inputs produce byte-identical outputs across
calls and through the full serialize/deserialize pipeline.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

from tinyquant_py_reference.codec.codec import Codec
from tinyquant_py_reference.codec.codec_config import CodecConfig
from tinyquant_py_reference.codec.compressed_vector import CompressedVector

if TYPE_CHECKING:
    from numpy.typing import NDArray

    from tinyquant_py_reference.codec.codebook import Codebook


def test_compress_deterministic_across_calls(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config: CodecConfig,
    trained_codebook_10k: Codebook,
    codec: Codec,
) -> None:
    """Same input and config produce byte-identical compressed output."""
    vector = gold_corpus_10k[0]

    result_a = codec.compress(vector, calibrated_config, trained_codebook_10k)
    result_b = codec.compress(vector, calibrated_config, trained_codebook_10k)

    assert result_a.to_bytes() == result_b.to_bytes()


def test_full_pipeline_deterministic(
    gold_corpus_10k: NDArray[np.float32],
    calibrated_config: CodecConfig,
    trained_codebook_10k: Codebook,
    codec: Codec,
) -> None:
    """Compress -> serialize -> deserialize -> decompress is deterministic."""
    vector = gold_corpus_10k[0]

    outputs: list[NDArray[np.float32]] = []
    for _ in range(2):
        compressed = codec.compress(vector, calibrated_config, trained_codebook_10k)
        raw = compressed.to_bytes()
        restored = CompressedVector.from_bytes(raw)
        decompressed = codec.decompress(
            restored,
            calibrated_config,
            trained_codebook_10k,
        )
        outputs.append(decompressed)

    np.testing.assert_array_equal(
        outputs[0],
        outputs[1],
        err_msg="Full pipeline produced different FP32 results across runs",
    )
