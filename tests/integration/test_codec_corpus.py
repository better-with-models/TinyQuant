"""Integration tests for Codec -> Corpus vector handoff (IB-01)."""

from __future__ import annotations

import numpy as np
import pytest
from numpy.typing import NDArray

from tinyquant.codec.codebook import Codebook
from tinyquant.codec.codec import Codec
from tinyquant.codec.codec_config import CodecConfig
from tinyquant.corpus.corpus import Corpus


@pytest.fixture()
def codec() -> Codec:
    """Shared stateless codec instance."""
    return Codec()


class TestCodecCorpusIntegration:
    """IB-01: CompressedVector from Codec integrates with Corpus."""

    def test_codec_output_accepted_by_corpus(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
        corpus_compress: Corpus,
    ) -> None:
        """Corpus stores the same compressed representation as Codec produces."""
        cv_direct = codec.compress(sample_vector, config_4bit, trained_codebook)

        corpus_compress.insert("direct-compare", sample_vector)
        entry = corpus_compress.get("direct-compare")

        assert entry.compressed.config_hash == cv_direct.config_hash
        assert np.array_equal(entry.compressed.indices, cv_direct.indices)
        assert entry.compressed.residual == cv_direct.residual

    def test_codec_config_hash_matches_corpus(
        self,
        config_4bit: CodecConfig,
        sample_vector: NDArray[np.float32],
        corpus_compress: Corpus,
    ) -> None:
        """Config hash propagates consistently through Corpus insert."""
        corpus_compress.insert("hash-check", sample_vector)
        entry = corpus_compress.get("hash-check")

        assert entry.config_hash == config_4bit.config_hash
        assert entry.compressed.config_hash == config_4bit.config_hash

    def test_corpus_decompress_uses_same_codec(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
        corpus_compress: Corpus,
    ) -> None:
        """Corpus.decompress() matches direct Codec round-trip."""
        cv = codec.compress(sample_vector, config_4bit, trained_codebook)
        direct_result = codec.decompress(cv, config_4bit, trained_codebook)

        corpus_compress.insert("decomp-compare", sample_vector)
        corpus_result = corpus_compress.decompress("decomp-compare")

        np.testing.assert_array_equal(direct_result, corpus_result)

    def test_batch_compress_integrates_with_batch_insert(
        self,
        codec: Codec,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vectors: NDArray[np.float32],
        corpus_compress: Corpus,
    ) -> None:
        """Batch codec compress produces same results as corpus batch insert."""
        batch = sample_vectors[:10]
        direct_batch = codec.compress_batch(batch, config_4bit, trained_codebook)

        vectors_dict = {f"batch-{i}": batch[i] for i in range(10)}
        corpus_compress.insert_batch(vectors_dict)

        for i in range(10):
            entry = corpus_compress.get(f"batch-{i}")
            assert np.array_equal(entry.compressed.indices, direct_batch[i].indices)
            assert entry.compressed.residual == direct_batch[i].residual
            assert entry.compressed.config_hash == direct_batch[i].config_hash
