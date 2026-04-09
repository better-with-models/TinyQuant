"""TinyQuant codec: compression and decompression primitives."""

from tinyquant_cpu.codec._errors import (
    CodebookIncompatibleError,
    ConfigMismatchError,
    DimensionMismatchError,
    DuplicateVectorError,
)
from tinyquant_cpu.codec.codebook import Codebook
from tinyquant_cpu.codec.codec import Codec, compress, decompress
from tinyquant_cpu.codec.codec_config import CodecConfig
from tinyquant_cpu.codec.compressed_vector import CompressedVector
from tinyquant_cpu.codec.rotation_matrix import RotationMatrix

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
