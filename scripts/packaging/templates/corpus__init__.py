"""TinyQuant corpus: aggregate root and vector lifecycle (Rust-backed)."""

import sys
import tinyquant_cpu  # noqa: F401

_core_corpus = sys.modules["tinyquant_cpu._core"].corpus

CompressionPolicy                     = _core_corpus.CompressionPolicy
CompressionPolicyViolationDetected    = _core_corpus.CompressionPolicyViolationDetected
Corpus                                = _core_corpus.Corpus
CorpusCreated                         = _core_corpus.CorpusCreated
CorpusDecompressed                    = _core_corpus.CorpusDecompressed
VectorEntry                           = _core_corpus.VectorEntry
VectorsInserted                       = _core_corpus.VectorsInserted

__all__ = [
    "CompressionPolicy",
    "CompressionPolicyViolationDetected",
    "Corpus",
    "CorpusCreated",
    "CorpusDecompressed",
    "VectorEntry",
    "VectorsInserted",
]
