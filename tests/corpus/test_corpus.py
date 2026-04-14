"""Tests for the Corpus aggregate root."""

from __future__ import annotations

import numpy as np
import pytest
from numpy.typing import NDArray

from tinyquant_py_reference.codec._errors import (
    DimensionMismatchError,
    DuplicateVectorError,
)
from tinyquant_py_reference.codec.codebook import Codebook
from tinyquant_py_reference.codec.codec_config import CodecConfig
from tinyquant_py_reference.corpus.compression_policy import CompressionPolicy
from tinyquant_py_reference.corpus.corpus import Corpus
from tinyquant_py_reference.corpus.events import (
    CorpusCreated,
    CorpusDecompressed,
    VectorsInserted,
)
from tinyquant_py_reference.corpus.vector_entry import VectorEntry

# ===========================================================================
# Creation tests
# ===========================================================================


class TestCreation:
    """Tests for Corpus construction."""

    def test_create_empty_corpus(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """Newly created corpus has zero vectors."""
        assert corpus_compress.vector_count == 0
        assert corpus_compress.is_empty is True

    def test_create_corpus_freezes_config(
        self,
        corpus_compress: Corpus,
        config_4bit: CodecConfig,
    ) -> None:
        """Codec config is the one passed at construction."""
        assert corpus_compress.codec_config is config_4bit

    def test_create_corpus_freezes_codebook(
        self,
        corpus_compress: Corpus,
        trained_codebook: Codebook,
    ) -> None:
        """Codebook is accessible after construction."""
        assert corpus_compress.codebook is trained_codebook

    def test_create_corpus_raises_corpus_created_event(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """CorpusCreated event with correct fields."""
        events = corpus_compress.pending_events()
        assert len(events) == 1
        event = events[0]
        assert isinstance(event, CorpusCreated)
        assert event.corpus_id == "test-corpus"
        assert event.compression_policy is CompressionPolicy.COMPRESS

    def test_pending_events_clears_after_collection(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """pending_events drains the list."""
        corpus_compress.pending_events()
        assert len(corpus_compress.pending_events()) == 0


# ===========================================================================
# Insert tests (COMPRESS policy)
# ===========================================================================


class TestInsert:
    """Tests for Corpus.insert with COMPRESS policy."""

    def test_insert_stores_vector(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """vector_count increments and vector is retrievable."""
        corpus_compress.insert("v1", sample_vector)
        assert corpus_compress.vector_count == 1
        assert corpus_compress.contains("v1")

    def test_insert_returns_vector_entry(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Insert returns a VectorEntry."""
        entry = corpus_compress.insert("v1", sample_vector)
        assert isinstance(entry, VectorEntry)
        assert entry.vector_id == "v1"

    def test_insert_compresses_with_compress_policy(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Under COMPRESS, a CompressedVector is created via codec."""
        entry = corpus_compress.insert("v1", sample_vector)
        assert entry.compressed is not None
        assert entry.has_residual is True

    def test_insert_config_hash_matches(
        self,
        corpus_compress: Corpus,
        config_4bit: CodecConfig,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Entry config_hash matches corpus config."""
        entry = corpus_compress.insert("v1", sample_vector)
        assert entry.config_hash == config_4bit.config_hash

    def test_insert_dimension_mismatch_raises(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """Wrong-dimension vector raises DimensionMismatchError."""
        wrong = np.zeros(32, dtype=np.float32)
        with pytest.raises(DimensionMismatchError):
            corpus_compress.insert("v1", wrong)

    def test_insert_duplicate_id_raises(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Same vector_id twice raises DuplicateVectorError."""
        corpus_compress.insert("v1", sample_vector)
        with pytest.raises(DuplicateVectorError):
            corpus_compress.insert("v1", sample_vector)

    def test_insert_sets_inserted_at(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Entry has a valid UTC timestamp."""
        entry = corpus_compress.insert("v1", sample_vector)
        assert entry.inserted_at.tzinfo is not None

    def test_insert_with_metadata(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Per-vector metadata is preserved."""
        entry = corpus_compress.insert("v1", sample_vector, {"source": "test"})
        assert entry.metadata == {"source": "test"}

    def test_insert_raises_vectors_inserted_event(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """VectorsInserted event after insert."""
        corpus_compress.pending_events()  # drain creation event
        corpus_compress.insert("v1", sample_vector)
        events = corpus_compress.pending_events()
        assert len(events) == 1
        event = events[0]
        assert isinstance(event, VectorsInserted)
        assert event.vector_ids == ("v1",)
        assert event.count == 1


# ===========================================================================
# Insert tests (PASSTHROUGH and FP16 policies)
# ===========================================================================


class TestInsertPolicies:
    """Tests for insert with non-COMPRESS policies."""

    def test_insert_stores_fp32_with_passthrough_policy(
        self,
        corpus_passthrough: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Under PASSTHROUGH, original FP32 is recoverable."""
        corpus_passthrough.insert("v1", sample_vector)
        result = corpus_passthrough.decompress("v1")
        assert result.dtype == np.float32
        assert np.array_equal(result, sample_vector)

    def test_insert_stores_fp16_with_fp16_policy(
        self,
        corpus_fp16: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Under FP16, vector is downcast then recoverable at FP16 precision."""
        corpus_fp16.insert("v1", sample_vector)
        result = corpus_fp16.decompress("v1")
        assert result.dtype == np.float32
        expected = sample_vector.astype(np.float16).astype(np.float32)
        assert np.allclose(result, expected)


# ===========================================================================
# Batch insert tests
# ===========================================================================


class TestInsertBatch:
    """Tests for Corpus.insert_batch."""

    def test_insert_batch_stores_all(
        self,
        corpus_compress: Corpus,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """N vectors produce N entries."""
        batch = {f"v{i}": sample_vectors[i] for i in range(5)}
        entries = corpus_compress.insert_batch(batch)
        assert len(entries) == 5
        assert corpus_compress.vector_count == 5

    def test_insert_batch_is_atomic(
        self,
        corpus_compress: Corpus,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """If one fails, none are stored."""
        corpus_compress.insert("v1", sample_vectors[0])
        batch = {
            "v2": sample_vectors[1],
            "v1": sample_vectors[2],  # duplicate
        }
        with pytest.raises(DuplicateVectorError):
            corpus_compress.insert_batch(batch)
        # Only the original v1 remains
        assert corpus_compress.vector_count == 1

    def test_insert_batch_raises_vectors_inserted_event(
        self,
        corpus_compress: Corpus,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Event lists all inserted IDs."""
        corpus_compress.pending_events()  # drain creation event
        batch = {f"v{i}": sample_vectors[i] for i in range(3)}
        corpus_compress.insert_batch(batch)
        events = corpus_compress.pending_events()
        assert len(events) == 1
        event = events[0]
        assert isinstance(event, VectorsInserted)
        assert event.count == 3
        assert set(event.vector_ids) == {"v0", "v1", "v2"}

    def test_insert_batch_empty_is_noop(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """Empty dict produces no error and no event."""
        corpus_compress.pending_events()  # drain creation event
        result = corpus_compress.insert_batch({})
        assert result == []
        assert corpus_compress.pending_events() == []


# ===========================================================================
# Get and contains tests
# ===========================================================================


class TestGetAndContains:
    """Tests for Corpus.get and Corpus.contains."""

    def test_get_returns_correct_entry(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Correct vector_id and compressed data."""
        corpus_compress.insert("v1", sample_vector)
        entry = corpus_compress.get("v1")
        assert entry.vector_id == "v1"

    def test_get_missing_raises_key_error(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """Unknown ID raises KeyError."""
        with pytest.raises(KeyError):
            corpus_compress.get("nonexistent")

    def test_contains_true_for_stored(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Inserted ID returns True."""
        corpus_compress.insert("v1", sample_vector)
        assert corpus_compress.contains("v1") is True

    def test_contains_false_for_missing(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """Unknown ID returns False."""
        assert corpus_compress.contains("v1") is False


# ===========================================================================
# Decompress tests
# ===========================================================================


class TestDecompress:
    """Tests for Corpus.decompress and decompress_all."""

    def test_decompress_single_returns_fp32(
        self,
        corpus_compress: Corpus,
        config_4bit: CodecConfig,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Correct dimension and float32 dtype."""
        corpus_compress.insert("v1", sample_vector)
        result = corpus_compress.decompress("v1")
        assert result.dtype == np.float32
        assert len(result) == config_4bit.dimension

    def test_decompress_missing_raises_key_error(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """Unknown ID raises KeyError."""
        with pytest.raises(KeyError):
            corpus_compress.decompress("nonexistent")

    def test_decompress_all_returns_all_vectors(
        self,
        corpus_compress: Corpus,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """All IDs present in result dict."""
        for i in range(3):
            corpus_compress.insert(f"v{i}", sample_vectors[i])
        result = corpus_compress.decompress_all()
        assert set(result.keys()) == {"v0", "v1", "v2"}
        for v in result.values():
            assert v.dtype == np.float32

    def test_decompress_all_raises_event(
        self,
        corpus_compress: Corpus,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """CorpusDecompressed event with correct count."""
        for i in range(3):
            corpus_compress.insert(f"v{i}", sample_vectors[i])
        corpus_compress.pending_events()  # drain
        corpus_compress.decompress_all()
        events = corpus_compress.pending_events()
        assert len(events) == 1
        event = events[0]
        assert isinstance(event, CorpusDecompressed)
        assert event.vector_count == 3


# ===========================================================================
# Remove tests
# ===========================================================================


class TestRemove:
    """Tests for Corpus.remove."""

    def test_remove_decrements_count(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """vector_count decreases after removal."""
        corpus_compress.insert("v1", sample_vector)
        assert corpus_compress.vector_count == 1
        corpus_compress.remove("v1")
        assert corpus_compress.vector_count == 0

    def test_remove_missing_raises_key_error(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """Unknown ID raises KeyError."""
        with pytest.raises(KeyError):
            corpus_compress.remove("nonexistent")

    def test_remove_makes_id_unavailable(
        self,
        corpus_compress: Corpus,
        sample_vector: NDArray[np.float32],
    ) -> None:
        """Contains returns False after removal."""
        corpus_compress.insert("v1", sample_vector)
        corpus_compress.remove("v1")
        assert corpus_compress.contains("v1") is False


# ===========================================================================
# Property tests
# ===========================================================================


class TestProperties:
    """Tests for Corpus properties."""

    def test_vector_count_reflects_state(
        self,
        corpus_compress: Corpus,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Count matches inserts minus removes."""
        corpus_compress.insert("v1", sample_vectors[0])
        corpus_compress.insert("v2", sample_vectors[1])
        assert corpus_compress.vector_count == 2
        corpus_compress.remove("v1")
        assert corpus_compress.vector_count == 1

    def test_is_empty_on_new_corpus(
        self,
        corpus_compress: Corpus,
    ) -> None:
        """True initially."""
        assert corpus_compress.is_empty is True

    def test_vector_ids_returns_all_ids(
        self,
        corpus_compress: Corpus,
        sample_vectors: NDArray[np.float32],
    ) -> None:
        """Frozenset of all stored IDs."""
        corpus_compress.insert("v1", sample_vectors[0])
        corpus_compress.insert("v2", sample_vectors[1])
        assert corpus_compress.vector_ids == frozenset({"v1", "v2"})
