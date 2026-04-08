"""TinyQuant codec: compression and decompression primitives."""

from tinyquant.codec._errors import (
    CodebookIncompatibleError,
    ConfigMismatchError,
    DimensionMismatchError,
)
from tinyquant.codec.codebook import Codebook
from tinyquant.codec.codec import Codec, compress, decompress
from tinyquant.codec.codec_config import CodecConfig
from tinyquant.codec.compressed_vector import CompressedVector
from tinyquant.codec.rotation_matrix import RotationMatrix

__all__ = [
    "Codebook",
    "CodebookIncompatibleError",
    "Codec",
    "CodecConfig",
    "CompressedVector",
    "ConfigMismatchError",
    "DimensionMismatchError",
    "RotationMatrix",
    "compress",
    "decompress",
]
