"""CompressedVector: immutable codec output value object."""

from __future__ import annotations

import math
import struct
from dataclasses import dataclass
from typing import TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:
    from numpy.typing import NDArray

    from tinyquant_cpu._types import ConfigHash

_FORMAT_VERSION: int = 0x01
_HASH_BYTES: int = 64
_HEADER_FORMAT: str = "<B64sIB"  # version, hash, dimension, bit_width
_HEADER_SIZE: int = struct.calcsize(_HEADER_FORMAT)
_VALID_BIT_WIDTHS: frozenset[int] = frozenset({2, 4, 8})


def _pack_indices(indices: NDArray[np.uint8], bit_width: int) -> bytes:
    """Bit-pack indices at *bit_width* bits per index, LSB-first."""
    if bit_width == 8:
        return bytes(indices)

    total_bits = len(indices) * bit_width
    n_bytes = math.ceil(total_bits / 8)
    buf = bytearray(n_bytes)
    bit_pos = 0
    for idx in indices:
        byte_idx = bit_pos // 8
        bit_off = bit_pos % 8
        buf[byte_idx] |= int(idx) << bit_off
        if bit_off + bit_width > 8:
            buf[byte_idx + 1] |= int(idx) >> (8 - bit_off)
        bit_pos += bit_width
    return bytes(buf)


def _unpack_indices(
    data: bytes,
    dimension: int,
    bit_width: int,
) -> NDArray[np.uint8]:
    """Unpack *dimension* indices from bit-packed *data*."""
    if bit_width == 8:
        return np.frombuffer(data[:dimension], dtype=np.uint8).copy()

    mask = (1 << bit_width) - 1
    out = np.empty(dimension, dtype=np.uint8)
    bit_pos = 0
    for i in range(dimension):
        byte_idx = bit_pos // 8
        bit_off = bit_pos % 8
        val = data[byte_idx] >> bit_off
        if bit_off + bit_width > 8:
            val |= data[byte_idx + 1] << (8 - bit_off)
        out[i] = val & mask
        bit_pos += bit_width
    return out


def _parse_header(data: bytes) -> tuple[int, str, int, int]:
    """Parse and return (version, config_hash, dimension, bit_width)."""
    if len(data) < _HEADER_SIZE:
        msg = f"data too short: expected at least {_HEADER_SIZE} bytes, got {len(data)}"
        raise ValueError(msg)
    version, hash_raw, dimension, bit_width = struct.unpack_from(
        _HEADER_FORMAT,
        data,
    )
    config_hash: str = hash_raw.rstrip(b"\x00").decode(
        "utf-8",
        errors="replace",
    )
    return version, config_hash, dimension, bit_width


def _validate_header(version: int, bit_width: int) -> None:
    """Validate version and bit_width from a parsed header."""
    if version != _FORMAT_VERSION:
        msg = f"unknown format version {version:#04x}, expected {_FORMAT_VERSION:#04x}"
        raise ValueError(msg)
    if bit_width not in _VALID_BIT_WIDTHS:
        msg = f"invalid bit_width {bit_width} in serialized data"
        raise ValueError(msg)


def _parse_residual(data: bytes, offset: int) -> tuple[bytes | None, int]:
    """Parse the residual section starting at *offset*."""
    residual_flag = data[offset]
    offset += 1
    if not residual_flag:
        return None, offset
    if len(data) < offset + 4:
        msg = "data truncated: missing residual length"
        raise ValueError(msg)
    (residual_len,) = struct.unpack_from("<I", data, offset)
    offset += 4
    if len(data) < offset + residual_len:
        msg = "data truncated: incomplete residual data"
        raise ValueError(msg)
    return data[offset : offset + residual_len], offset + residual_len


@dataclass(frozen=True)
class CompressedVector:
    """Immutable output of a codec compression pass.

    Carries quantized indices, optional residual data, and a config hash
    linking it to the originating configuration.
    """

    indices: NDArray[np.uint8]
    """One quantized index per dimension."""

    residual: bytes | None
    """Residual correction data, or ``None`` if residual was disabled."""

    config_hash: ConfigHash
    """Fingerprint of the ``CodecConfig`` that produced this vector."""

    dimension: int
    """Length of ``indices``; stored explicitly for fast validation."""

    bit_width: int
    """Quantization bit width (2, 4, or 8)."""

    def __post_init__(self) -> None:
        """Validate fields and freeze the indices array."""
        if self.bit_width not in _VALID_BIT_WIDTHS:
            msg = (
                f"bit_width must be one of "
                f"{sorted(_VALID_BIT_WIDTHS)}, got {self.bit_width}"
            )
            raise ValueError(msg)
        if len(self.indices) != self.dimension:
            msg = (
                f"indices length ({len(self.indices)}) must match "
                f"dimension ({self.dimension})"
            )
            raise ValueError(msg)
        frozen = self.indices.copy()
        frozen.flags.writeable = False
        object.__setattr__(self, "indices", frozen)

    @property
    def has_residual(self) -> bool:
        """Whether this vector carries residual correction data."""
        return self.residual is not None

    @property
    def size_bytes(self) -> int:
        """Approximate storage footprint in bytes."""
        packed = math.ceil(self.dimension * self.bit_width / 8)
        return packed + (len(self.residual) if self.residual else 0)

    def to_bytes(self) -> bytes:
        """Serialize to a compact versioned binary representation.

        Returns:
            Binary bytes suitable for persistent storage.
        """
        hash_bytes = self.config_hash.encode("utf-8")[:_HASH_BYTES].ljust(
            _HASH_BYTES,
            b"\x00",
        )
        header = struct.pack(
            _HEADER_FORMAT,
            _FORMAT_VERSION,
            hash_bytes,
            self.dimension,
            self.bit_width,
        )
        packed = _pack_indices(self.indices, self.bit_width)
        residual_flag = b"\x01" if self.residual is not None else b"\x00"
        parts = [header, packed, residual_flag]
        if self.residual is not None:
            parts.append(struct.pack("<I", len(self.residual)))
            parts.append(self.residual)
        return b"".join(parts)

    @classmethod
    def from_bytes(cls, data: bytes) -> CompressedVector:
        """Deserialize from a compact binary representation.

        Args:
            data: Binary data produced by :meth:`to_bytes`.

        Raises:
            ValueError: If the data is truncated, has an unknown version,
                or is otherwise malformed.
        """
        version, config_hash, dimension, bit_width = _parse_header(data)
        _validate_header(version, bit_width)

        packed_len = math.ceil(dimension * bit_width / 8)
        offset = _HEADER_SIZE
        if len(data) < offset + packed_len + 1:
            msg = "data truncated: missing packed indices or residual flag"
            raise ValueError(msg)

        indices = _unpack_indices(
            data[offset : offset + packed_len],
            dimension,
            bit_width,
        )
        offset += packed_len

        residual, offset = _parse_residual(data, offset)

        return cls(
            indices=indices,
            residual=residual,
            config_hash=config_hash,
            dimension=dimension,
            bit_width=bit_width,
        )
