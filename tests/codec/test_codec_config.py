"""Unit tests for CodecConfig value object."""

from __future__ import annotations

import dataclasses

import pytest
from hypothesis import given
from hypothesis import strategies as st

from tinyquant.codec.codec_config import SUPPORTED_BIT_WIDTHS, CodecConfig

# ---------------------------------------------------------------------------
# Construction and validation
# ---------------------------------------------------------------------------


class TestConstruction:
    """Tests for CodecConfig construction and validation."""

    def test_valid_config_creates_successfully(self) -> None:
        """4-bit, seed 42, dim 768 constructs without error."""
        config = CodecConfig(bit_width=4, seed=42, dimension=768)
        assert config.bit_width == 4
        assert config.seed == 42
        assert config.dimension == 768
        assert config.residual_enabled is True

    def test_unsupported_bit_width_raises(self) -> None:
        """bit_width=3 raises ValueError."""
        with pytest.raises(ValueError, match="bit_width"):
            CodecConfig(bit_width=3, seed=42, dimension=768)

    def test_negative_seed_raises(self) -> None:
        """seed=-1 raises ValueError."""
        with pytest.raises(ValueError, match="seed"):
            CodecConfig(bit_width=4, seed=-1, dimension=768)

    def test_zero_dimension_raises(self) -> None:
        """dimension=0 raises ValueError."""
        with pytest.raises(ValueError, match="dimension"):
            CodecConfig(bit_width=4, seed=42, dimension=0)

    def test_negative_dimension_raises(self) -> None:
        """dimension=-5 raises ValueError."""
        with pytest.raises(ValueError, match="dimension"):
            CodecConfig(bit_width=4, seed=42, dimension=-5)

    def test_residual_enabled_defaults_true(self) -> None:
        """Omitting residual_enabled defaults to True."""
        config = CodecConfig(bit_width=4, seed=42, dimension=768)
        assert config.residual_enabled is True


# ---------------------------------------------------------------------------
# Immutability
# ---------------------------------------------------------------------------


class TestImmutability:
    """Tests for frozen dataclass immutability."""

    def test_frozen_fields_reject_assignment(self) -> None:
        """Assigning to any field raises FrozenInstanceError."""
        config = CodecConfig(bit_width=4, seed=42, dimension=768)
        with pytest.raises(dataclasses.FrozenInstanceError):
            config.bit_width = 8  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Properties
# ---------------------------------------------------------------------------


class TestProperties:
    """Tests for computed properties."""

    def test_num_codebook_entries_4bit(self) -> None:
        """4-bit config has 16 codebook entries."""
        config = CodecConfig(bit_width=4, seed=42, dimension=768)
        assert config.num_codebook_entries == 16

    def test_num_codebook_entries_2bit(self) -> None:
        """2-bit config has 4 codebook entries."""
        config = CodecConfig(bit_width=2, seed=42, dimension=768)
        assert config.num_codebook_entries == 4

    def test_num_codebook_entries_8bit(self) -> None:
        """8-bit config has 256 codebook entries."""
        config = CodecConfig(bit_width=8, seed=42, dimension=768)
        assert config.num_codebook_entries == 256

    def test_config_hash_is_deterministic(self) -> None:
        """Same fields produce same hash across calls."""
        config = CodecConfig(bit_width=4, seed=42, dimension=768)
        assert config.config_hash == config.config_hash

        config2 = CodecConfig(bit_width=4, seed=42, dimension=768)
        assert config.config_hash == config2.config_hash

    def test_config_hash_differs_for_different_fields(self) -> None:
        """Changing any field changes the hash."""
        base = CodecConfig(bit_width=4, seed=42, dimension=768)
        different_bw = CodecConfig(bit_width=2, seed=42, dimension=768)
        different_seed = CodecConfig(bit_width=4, seed=99, dimension=768)
        different_dim = CodecConfig(bit_width=4, seed=42, dimension=128)
        different_res = CodecConfig(
            bit_width=4, seed=42, dimension=768, residual_enabled=False
        )

        assert base.config_hash != different_bw.config_hash
        assert base.config_hash != different_seed.config_hash
        assert base.config_hash != different_dim.config_hash
        assert base.config_hash != different_res.config_hash


# ---------------------------------------------------------------------------
# Equality and hashing
# ---------------------------------------------------------------------------


class TestEqualityAndHashing:
    """Tests for structural equality and hash consistency."""

    def test_equal_configs_are_equal(self) -> None:
        """Two configs with same fields are ==."""
        a = CodecConfig(bit_width=4, seed=42, dimension=768)
        b = CodecConfig(bit_width=4, seed=42, dimension=768)
        assert a == b

    def test_equal_configs_have_same_hash(self) -> None:
        """Hash consistency for dict/set usage."""
        a = CodecConfig(bit_width=4, seed=42, dimension=768)
        b = CodecConfig(bit_width=4, seed=42, dimension=768)
        assert hash(a) == hash(b)

    def test_different_configs_are_not_equal(self) -> None:
        """Different bit_width means not ==."""
        a = CodecConfig(bit_width=4, seed=42, dimension=768)
        b = CodecConfig(bit_width=2, seed=42, dimension=768)
        assert a != b


# ---------------------------------------------------------------------------
# Property-based (hypothesis)
# ---------------------------------------------------------------------------


class TestHypothesis:
    """Property-based tests using hypothesis."""

    @given(
        bit_width=st.sampled_from(sorted(SUPPORTED_BIT_WIDTHS)),
        seed=st.integers(min_value=0, max_value=10_000),
        dimension=st.integers(min_value=1, max_value=2048),
    )
    def test_any_supported_config_is_valid(
        self,
        bit_width: int,
        seed: int,
        dimension: int,
    ) -> None:
        """All combinations of supported params construct successfully."""
        config = CodecConfig(bit_width=bit_width, seed=seed, dimension=dimension)
        assert config.num_codebook_entries == 2**bit_width
        assert isinstance(config.config_hash, str)
        assert len(config.config_hash) > 0
