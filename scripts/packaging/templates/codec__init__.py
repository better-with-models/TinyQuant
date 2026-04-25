"""TinyQuant codec: compression and decompression primitives (Rust-backed)."""

import sys
# Ensure the extension is loaded before we pull sub-attrs off it.
import tinyquant_cpu  # noqa: F401  -- triggers _selector.load_core()

_core_codec = sys.modules["tinyquant_cpu._core"].codec

CodebookIncompatibleError = _core_codec.CodebookIncompatibleError
ConfigMismatchError       = _core_codec.ConfigMismatchError
DimensionMismatchError    = _core_codec.DimensionMismatchError
DuplicateVectorError      = _core_codec.DuplicateVectorError
Codebook                  = _core_codec.Codebook
Codec                     = _core_codec.Codec
CodecConfig               = _core_codec.CodecConfig
CompressedVector          = _core_codec.CompressedVector
RotationMatrix            = _core_codec.RotationMatrix
compress                  = _core_codec.compress
decompress                = _core_codec.decompress

__all__ = [
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
]
