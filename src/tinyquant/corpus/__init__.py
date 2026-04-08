"""TinyQuant corpus: aggregate root and vector lifecycle management."""

from tinyquant.corpus.compression_policy import CompressionPolicy
from tinyquant.corpus.corpus import Corpus
from tinyquant.corpus.events import (
    CompressionPolicyViolationDetected,
    CorpusCreated,
    CorpusDecompressed,
    VectorsInserted,
)
from tinyquant.corpus.vector_entry import VectorEntry

__all__ = [
    "CompressionPolicy",
    "CompressionPolicyViolationDetected",
    "Corpus",
    "CorpusCreated",
    "CorpusDecompressed",
    "VectorEntry",
    "VectorsInserted",
]
