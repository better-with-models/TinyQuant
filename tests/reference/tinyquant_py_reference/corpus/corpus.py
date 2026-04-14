"""Corpus: aggregate root for vector lifecycle management."""

from __future__ import annotations

from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

import numpy as np

from tinyquant_py_reference.codec._errors import (
    DimensionMismatchError,
    DuplicateVectorError,
)
from tinyquant_py_reference.codec.codec import Codec
from tinyquant_py_reference.codec.compressed_vector import CompressedVector
from tinyquant_py_reference.corpus.compression_policy import CompressionPolicy
from tinyquant_py_reference.corpus.events import (
    CorpusCreated,
    CorpusDecompressed,
    VectorsInserted,
)
from tinyquant_py_reference.corpus.vector_entry import VectorEntry

if TYPE_CHECKING:
    from numpy.typing import NDArray

    from tinyquant_py_reference.codec.codebook import Codebook
    from tinyquant_py_reference.codec.codec_config import CodecConfig


class Corpus:
    """Aggregate root: the primary consistency boundary for stored vectors.

    All vector mutations flow through this class. The corpus owns
    compression, storage, retrieval, and decompression of vectors.
    """

    def __init__(
        self,
        corpus_id: str,
        codec_config: CodecConfig,
        codebook: Codebook,
        compression_policy: CompressionPolicy,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        """Initialize a corpus aggregate.

        Args:
            corpus_id: Unique identity for this corpus.
            codec_config: Frozen configuration for all vectors.
            codebook: Frozen codebook matching the config.
            compression_policy: Governs write-path behavior.
            metadata: Optional per-corpus metadata.
        """
        self._corpus_id = corpus_id
        self._codec_config = codec_config
        self._codebook = codebook
        self._compression_policy = compression_policy
        self._metadata: dict[str, Any] = metadata if metadata is not None else {}
        self._vectors: dict[str, VectorEntry] = {}
        self._codec = Codec()
        self._pending_events: list[
            CorpusCreated | VectorsInserted | CorpusDecompressed
        ] = []

        self._pending_events.append(
            CorpusCreated(
                corpus_id=corpus_id,
                codec_config=codec_config,
                compression_policy=compression_policy,
                timestamp=datetime.now(UTC),
            )
        )

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def corpus_id(self) -> str:
        """Unique corpus identity."""
        return self._corpus_id

    @property
    def codec_config(self) -> CodecConfig:
        """Frozen codec configuration."""
        return self._codec_config

    @property
    def codebook(self) -> Codebook:
        """Frozen codebook."""
        return self._codebook

    @property
    def compression_policy(self) -> CompressionPolicy:
        """Write-path compression policy."""
        return self._compression_policy

    @property
    def metadata(self) -> dict[str, Any]:
        """Consumer-defined metadata."""
        return self._metadata

    @property
    def vector_count(self) -> int:
        """Number of stored vector entries."""
        return len(self._vectors)

    @property
    def is_empty(self) -> bool:
        """True if no vectors are stored."""
        return self.vector_count == 0

    @property
    def vector_ids(self) -> frozenset[str]:
        """Set of all stored vector IDs."""
        return frozenset(self._vectors.keys())

    # ------------------------------------------------------------------
    # Event collection
    # ------------------------------------------------------------------

    def pending_events(
        self,
    ) -> list[CorpusCreated | VectorsInserted | CorpusDecompressed]:
        """Return and clear the pending event list."""
        events = list(self._pending_events)
        self._pending_events.clear()
        return events

    # ------------------------------------------------------------------
    # Insert
    # ------------------------------------------------------------------

    def insert(
        self,
        vector_id: str,
        vector: NDArray[np.float32],
        metadata: dict[str, Any] | None = None,
    ) -> VectorEntry:
        """Compress and store a single vector.

        Args:
            vector_id: Unique identifier within the corpus.
            vector: FP32 array of shape ``(dimension,)``.
            metadata: Optional per-vector metadata.

        Returns:
            The newly stored :class:`VectorEntry`.

        Raises:
            DimensionMismatchError: If vector dimension does not match config.
            DuplicateVectorError: If vector_id already exists.
        """
        self._validate_insert(vector_id, vector)
        compressed = self._compress_vector(vector)
        now = datetime.now(UTC)
        entry = VectorEntry(vector_id, compressed, now, metadata)
        self._vectors[vector_id] = entry
        self._pending_events.append(
            VectorsInserted(
                corpus_id=self._corpus_id,
                vector_ids=(vector_id,),
                count=1,
                timestamp=now,
            )
        )
        return entry

    def insert_batch(
        self,
        vectors: dict[str, NDArray[np.float32]],
        metadata: dict[str, dict[str, Any]] | None = None,
    ) -> list[VectorEntry]:
        """Compress and store multiple vectors atomically.

        Either all vectors are inserted or none are.

        Args:
            vectors: Mapping of vector_id to FP32 array.
            metadata: Mapping of vector_id to metadata dict (optional).

        Returns:
            All newly stored entries.

        Raises:
            DimensionMismatchError: If any vector has wrong dimension.
            DuplicateVectorError: If any vector_id already exists.
        """
        if not vectors:
            return []

        # Validate all upfront before any mutation.
        for vid, vec in vectors.items():
            self._validate_insert(vid, vec)

        now = datetime.now(UTC)
        entries: list[VectorEntry] = []
        for vid, vec in vectors.items():
            compressed = self._compress_vector(vec)
            meta = (metadata or {}).get(vid)
            entry = VectorEntry(vid, compressed, now, meta)
            self._vectors[vid] = entry
            entries.append(entry)

        self._pending_events.append(
            VectorsInserted(
                corpus_id=self._corpus_id,
                vector_ids=tuple(vectors.keys()),
                count=len(vectors),
                timestamp=now,
            )
        )
        return entries

    # ------------------------------------------------------------------
    # Retrieval
    # ------------------------------------------------------------------

    def get(self, vector_id: str) -> VectorEntry:
        """Retrieve a single vector entry by ID.

        Raises:
            KeyError: If vector_id is not found.
        """
        return self._vectors[vector_id]

    def contains(self, vector_id: str) -> bool:
        """Check if a vector ID exists in the corpus."""
        return vector_id in self._vectors

    # ------------------------------------------------------------------
    # Decompression
    # ------------------------------------------------------------------

    def decompress(self, vector_id: str) -> NDArray[np.float32]:
        """Decompress a single stored vector to FP32.

        Raises:
            KeyError: If vector_id is not found.
        """
        entry = self._vectors[vector_id]
        return self._decompress_entry(entry)

    def decompress_all(self) -> dict[str, NDArray[np.float32]]:
        """Batch decompress all stored vectors to FP32.

        Returns:
            Mapping of vector_id to FP32 array.
        """
        result = {
            vid: self._decompress_entry(entry) for vid, entry in self._vectors.items()
        }
        self._pending_events.append(
            CorpusDecompressed(
                corpus_id=self._corpus_id,
                vector_count=len(result),
                timestamp=datetime.now(UTC),
            )
        )
        return result

    # ------------------------------------------------------------------
    # Removal
    # ------------------------------------------------------------------

    def remove(self, vector_id: str) -> None:
        """Remove a vector entry from the corpus.

        Raises:
            KeyError: If vector_id is not found.
        """
        del self._vectors[vector_id]

    # ------------------------------------------------------------------
    # Private helpers
    # ------------------------------------------------------------------

    def _validate_insert(
        self,
        vector_id: str,
        vector: NDArray[np.float32],
    ) -> None:
        """Validate dimension and uniqueness before insertion."""
        if len(vector) != self._codec_config.dimension:
            msg = (
                f"vector length {len(vector)} does not match "
                f"config dimension {self._codec_config.dimension}"
            )
            raise DimensionMismatchError(msg)
        if vector_id in self._vectors:
            msg = f"vector_id {vector_id!r} already exists in corpus"
            raise DuplicateVectorError(msg)

    def _compress_vector(
        self,
        vector: NDArray[np.float32],
    ) -> CompressedVector:
        """Compress a vector according to the corpus policy."""
        if self._compression_policy is CompressionPolicy.COMPRESS:
            return self._codec.compress(vector, self._codec_config, self._codebook)

        if self._compression_policy is CompressionPolicy.FP16:
            fp16 = vector.astype(np.float16)
            indices = np.frombuffer(fp16.tobytes(), dtype=np.uint8).copy()
            return CompressedVector(
                indices=indices,
                residual=None,
                config_hash=self._codec_config.config_hash,
                dimension=len(indices),
                bit_width=8,
            )

        # PASSTHROUGH: store FP32 as raw bytes wrapped in uint8.
        indices = np.frombuffer(vector.tobytes(), dtype=np.uint8).copy()
        return CompressedVector(
            indices=indices,
            residual=None,
            config_hash=self._codec_config.config_hash,
            dimension=len(indices),
            bit_width=8,
        )

    def _decompress_entry(self, entry: VectorEntry) -> NDArray[np.float32]:
        """Decompress a single entry according to the corpus policy."""
        if self._compression_policy is CompressionPolicy.COMPRESS:
            return self._codec.decompress(
                entry.compressed, self._codec_config, self._codebook
            )

        if self._compression_policy is CompressionPolicy.FP16:
            return (
                np.frombuffer(entry.compressed.indices.tobytes(), dtype=np.float16)
                .astype(np.float32)
                .copy()
            )

        # PASSTHROUGH: reconstruct FP32 from raw uint8 bytes.
        return np.frombuffer(
            entry.compressed.indices.tobytes(), dtype=np.float32
        ).copy()
