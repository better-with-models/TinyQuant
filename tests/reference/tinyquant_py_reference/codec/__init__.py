"""TinyQuant codec: compression and decompression primitives."""

from tinyquant_py_reference.codec._errors import (
    CodebookIncompatibleError,
    ConfigMismatchError,
    DimensionMismatchError,
    DuplicateVectorError,
)
from tinyquant_py_reference.codec.codebook import Codebook
from tinyquant_py_reference.codec.codec import Codec, compress, decompress
from tinyquant_py_reference.codec.codec_config import CodecConfig
from tinyquant_py_reference.codec.compressed_vector import CompressedVector
from tinyquant_py_reference.codec.rotation_matrix import RotationMatrix

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
