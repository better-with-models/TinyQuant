"""CodecConfig: immutable configuration value object for TinyQuant codec."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from tinyquant_py_reference._types import ConfigHash

SUPPORTED_BIT_WIDTHS: frozenset[int] = frozenset({2, 4, 8})
"""Bit widths that TinyQuant supports for scalar quantization."""


@dataclass(frozen=True)
class CodecConfig:
    """Immutable configuration snapshot that fully determines codec behavior.

    Two configs with identical fields are interchangeable.
    """

    bit_width: int
    """Quantization bit width; must be in :data:`SUPPORTED_BIT_WIDTHS`."""

    seed: int
    """Non-negative seed controlling rotation matrix generation."""

    dimension: int
    """Positive integer; expected input vector dimensionality."""

    residual_enabled: bool = True
    """Whether stage-2 residual correction is active."""

    def __post_init__(self) -> None:
        """Validate all field invariants."""
        if self.bit_width not in SUPPORTED_BIT_WIDTHS:
            msg = (
                f"bit_width must be one of {sorted(SUPPORTED_BIT_WIDTHS)}, "
                f"got {self.bit_width}"
            )
            raise ValueError(msg)
        if self.seed < 0:
            msg = f"seed must be >= 0, got {self.seed}"
            raise ValueError(msg)
        if self.dimension <= 0:
            msg = f"dimension must be > 0, got {self.dimension}"
            raise ValueError(msg)

    @property
    def num_codebook_entries(self) -> int:
        """Number of quantization levels: ``2 ** bit_width``."""
        return int(2**self.bit_width)

    @property
    def config_hash(self) -> ConfigHash:
        """Deterministic hash of all fields for traceability."""
        canonical = (
            f"CodecConfig("
            f"bit_width={self.bit_width},"
            f"seed={self.seed},"
            f"dimension={self.dimension},"
            f"residual_enabled={self.residual_enabled})"
        )
        return hashlib.sha256(
            canonical.encode(),
            usedforsecurity=False,
        ).hexdigest()
