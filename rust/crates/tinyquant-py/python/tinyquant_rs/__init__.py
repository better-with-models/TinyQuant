"""tinyquant_rs: Rust-backed drop-in replacement for tinyquant_cpu.

This module re-exports every symbol advertised by the pyo3 extension module
(`tinyquant_rs._core`) so that downstream code can swap the import:

    # Before (pure-Python reference)
    from tinyquant_cpu.codec import CodecConfig, Codec

    # After (Rust wheel)
    from tinyquant_rs.codec import CodecConfig, Codec
"""

from __future__ import annotations

from tinyquant_rs._core import (  # noqa: F401
    # Codec surface
    Codebook,
    CodebookIncompatibleError,
    Codec,
    CodecConfig,
    CompressedVector,
    ConfigMismatchError,
    DimensionMismatchError,
    DuplicateVectorError,
    RotationMatrix,
    compress,
    decompress,
    # Corpus surface
    CompressionPolicy,
    CompressionPolicyViolationDetected,
    Corpus,
    CorpusCreated,
    CorpusDecompressed,
    VectorEntry,
    VectorsInserted,
    # Backend surface
    BruteForceBackend,
    SearchBackend,
    SearchResult,
    __version__,
)
from tinyquant_rs._core import backend as _backend_mod
from tinyquant_rs._core import codec as _codec_mod
from tinyquant_rs._core import corpus as _corpus_mod

codec = _codec_mod
corpus = _corpus_mod
backend = _backend_mod

__all__ = [
    "__version__",
    # Codec
    "Codebook",
    "CodebookIncompatibleError",
    "Codec",
    "CodecConfig",
    "CompressedVector",
    "ConfigMismatchError",
    "DimensionMismatchError",
    "DuplicateVectorError",
    "RotationMatrix",
    "compress",
    "decompress",
    # Corpus
    "CompressionPolicy",
    "CompressionPolicyViolationDetected",
    "Corpus",
    "CorpusCreated",
    "CorpusDecompressed",
    "VectorEntry",
    "VectorsInserted",
    # Backend
    "BruteForceBackend",
    "SearchBackend",
    "SearchResult",
    # Sub-modules
    "codec",
    "corpus",
    "backend",
]

# -------------------------------------------------------------------------
# Optional parity guard — warn when tinyquant_cpu is unavailable or lags
# the version we claim byte-parity with.
# -------------------------------------------------------------------------

_MIN_MATCHING_CPU = (0, 1, 1)

try:  # pragma: no cover - import guard, not exercised on the hot path.
    import tinyquant_cpu as _py

    _cpu_version = tuple(int(x) for x in _py.__version__.split("."))
    if _cpu_version < _MIN_MATCHING_CPU:
        import warnings

        warnings.warn(
            (
                f"tinyquant_cpu {_py.__version__} is older than the parity target "
                f"{'.'.join(map(str, _MIN_MATCHING_CPU))}; behavioral parity is "
                "not guaranteed."
            ),
            stacklevel=1,
        )
except ImportError:
    # tinyquant_cpu is an optional companion; its absence is not an error.
    pass
