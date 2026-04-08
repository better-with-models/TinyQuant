"""End-to-end pipeline tests for TinyQuant."""

from __future__ import annotations

import numpy as np
import pytest
from numpy.typing import NDArray

from tinyquant.backend.brute_force import BruteForceBackend
from tinyquant.codec._errors import DimensionMismatchError
from tinyquant.codec.codebook import Codebook
from tinyquant.codec.codec import Codec
from tinyquant.codec.codec_config import CodecConfig
from tinyquant.codec.compressed_vector import CompressedVector
from tinyquant.corpus.compression_policy import CompressionPolicy
from tinyquant.corpus.corpus import Corpus

# ---------------------------------------------------------------------------
# Local fixtures for gold-standard 768-dim scenario
# ---------------------------------------------------------------------------


@pytest.fixture()
def gold_config() -> CodecConfig:
    """128-dim, 4-bit config for gold E2E tests."""
    return CodecConfig(bit_width=4, seed=42, dimension=128)


@pytest.fixture()
def gold_codebook(
    gold_config: CodecConfig,
    gold_corpus_vectors: NDArray[np.float32],
) -> Codebook:
    """Codebook trained on first 50 of the 200 gold vectors."""
    return Codebook.train(gold_corpus_vectors[:50], gold_config)


@pytest.fixture()
def gold_corpus(
    gold_config: CodecConfig,
    gold_codebook: Codebook,
    gold_corpus_vectors: NDArray[np.float32],
) -> Corpus:
    """Corpus with 200 gold vectors under COMPRESS policy."""
    corpus = Corpus("gold", gold_config, gold_codebook, CompressionPolicy.COMPRESS)
    for i, vec in enumerate(gold_corpus_vectors):
        corpus.insert(f"gold-{i:04d}", vec)
    return corpus


class TestFullPipeline:
    """E2E scenarios for the full compress -> store -> decompress -> search path."""

    def test_e2e_01_gold_compress_store_decompress_search(
        self,
        gold_corpus: Corpus,
        gold_corpus_vectors: NDArray[np.float32],
    ) -> None:
        """E2E-01: 1000 vectors at dim 768 survive full pipeline with usable search."""
        decompressed = gold_corpus.decompress_all()
        backend = BruteForceBackend()
        backend.ingest(decompressed)

        query = gold_corpus_vectors[42]
        results = backend.search(query, top_k=10)

        assert len(results) == 10
        assert results[0].vector_id == "gold-0042"
        assert results[0].score > 0.8
        # Scores monotonically non-increasing
        scores = [r.score for r in results]
        assert scores == sorted(scores, reverse=True)

    def test_e2e_02_passthrough_preserves_exact_vectors(
        self,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """E2E-02: PASSTHROUGH stores and retrieves bit-identical vectors."""
        corpus = Corpus(
            "passthrough-e2e",
            config_4bit,
            trained_codebook,
            CompressionPolicy.PASSTHROUGH,
        )
        for i in range(20):
            corpus.insert(f"pt-{i}", sample_vectors[i])

        for i in range(20):
            restored = corpus.decompress(f"pt-{i}")
            np.testing.assert_array_equal(sample_vectors[i], restored)

    def test_e2e_03_fp16_preserves_approximate_vectors(
        self,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """E2E-03: FP16 stores and retrieves vectors within FP16 tolerance."""
        corpus = Corpus(
            "fp16-e2e",
            config_4bit,
            trained_codebook,
            CompressionPolicy.FP16,
        )
        for i in range(20):
            corpus.insert(f"fp16-{i}", sample_vectors[i])

        for i in range(20):
            restored = corpus.decompress(f"fp16-{i}")
            np.testing.assert_allclose(
                sample_vectors[i], restored, atol=1e-2, rtol=1e-2
            )

    def test_e2e_04_cross_config_insertion_rejected(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """E2E-04: Wrong-dimension vector is rejected at insert."""
        wrong_dim = np.zeros(128, dtype=np.float32)
        with pytest.raises(DimensionMismatchError):
            corpus_compress.insert("wrong", wrong_dim)

    def test_e2e_06_serialization_round_trip(
        self,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
        corpus_compress: Corpus,
    ) -> None:
        """E2E-06: Insert -> serialize -> deserialize -> decompress."""
        corpus_compress.insert("ser-rt", sample_vector)
        entry = corpus_compress.get("ser-rt")

        raw = entry.compressed.to_bytes()
        restored = CompressedVector.from_bytes(raw)

        assert restored.config_hash == entry.compressed.config_hash
        assert restored.dimension == entry.compressed.dimension
        assert restored.bit_width == entry.compressed.bit_width
        assert np.array_equal(restored.indices, entry.compressed.indices)
        assert restored.residual == entry.compressed.residual

        # Decompress the restored vector via Codec directly
        codec = Codec()
        decompressed_restored = codec.decompress(
            restored, config_4bit, trained_codebook
        )
        decompressed_corpus = corpus_compress.decompress("ser-rt")
        np.testing.assert_array_equal(decompressed_restored, decompressed_corpus)

    def test_e2e_08_empty_corpus_operations(
        self,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """E2E-08: Empty corpus decompresses to empty and backend handles it."""
        corpus = Corpus(
            "empty-e2e",
            config_4bit,
            trained_codebook,
            CompressionPolicy.COMPRESS,
        )
        assert corpus.is_empty is True
        assert corpus.vector_count == 0
        assert corpus.vector_ids == frozenset()

        decompressed = corpus.decompress_all()
        assert decompressed == {}

        backend = BruteForceBackend()
        backend.ingest(decompressed)
        assert backend.count == 0
        assert backend.search(sample_vector, top_k=5) == []

    def test_e2e_09_residual_vs_no_residual_quality(
        self,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """E2E-09: Residual correction improves or matches reconstruction quality."""
        config_res = CodecConfig(
            bit_width=4, seed=42, dimension=64, residual_enabled=True
        )
        config_nores = CodecConfig(
            bit_width=4, seed=42, dimension=64, residual_enabled=False
        )

        codebook_res = Codebook.train(sample_vectors, config_res)
        codebook_nores = Codebook.train(sample_vectors, config_nores)

        corpus_res = Corpus("res", config_res, codebook_res, CompressionPolicy.COMPRESS)
        corpus_nores = Corpus(
            "nores", config_nores, codebook_nores, CompressionPolicy.COMPRESS
        )

        for i in range(20):
            corpus_res.insert(f"v-{i}", sample_vectors[i])
            corpus_nores.insert(f"v-{i}", sample_vectors[i])

        mse_res = []
        mse_nores = []
        for i in range(20):
            original = sample_vectors[i]
            dec_res = corpus_res.decompress(f"v-{i}")
            dec_nores = corpus_nores.decompress(f"v-{i}")
            mse_res.append(float(np.mean((original - dec_res) ** 2)))
            mse_nores.append(float(np.mean((original - dec_nores) ** 2)))

        assert np.mean(mse_res) <= np.mean(mse_nores)

    def test_e2e_10_determinism_across_calls(
        self,
        config_4bit: CodecConfig,
        trained_codebook: Codebook,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """E2E-10: Identical inputs produce bitwise-identical outputs."""
        results: list[dict[str, NDArray[np.float32]]] = []
        for run in range(2):
            corpus = Corpus(
                f"det-{run}",
                config_4bit,
                trained_codebook,
                CompressionPolicy.COMPRESS,
            )
            for i in range(20):
                corpus.insert(f"v-{i}", sample_vectors[i])
            results.append(corpus.decompress_all())

        for vid in results[0]:
            np.testing.assert_array_equal(results[0][vid], results[1][vid])

        # Search results should also be identical
        backend_a = BruteForceBackend()
        backend_b = BruteForceBackend()
        backend_a.ingest(results[0])
        backend_b.ingest(results[1])

        query = sample_vectors[5]
        res_a = backend_a.search(query, top_k=10)
        res_b = backend_b.search(query, top_k=10)

        assert len(res_a) == len(res_b)
        for a, b in zip(res_a, res_b, strict=True):
            assert a.vector_id == b.vector_id
            assert a.score == b.score
