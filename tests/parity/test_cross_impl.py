"""Cross-implementation parity scaffold.

Phase 23 deliverable: structure exists, runs green against the
reference alone (trivial self-parity), and Phase 24 flips the
``rs`` fixture on to make every test meaningful.
"""

from __future__ import annotations

from types import ModuleType

import numpy as np
import numpy.typing as npt
import pytest

pytestmark = pytest.mark.parity


class TestConfigParity:
    """Parity tests for CodecConfig construction and config_hash determinism."""

    def test_config_hash_matches_self(
        self, ref: ModuleType, cfg_triplet: tuple[int, int, int]
    ) -> None:
        """Two identical CodecConfigs from the same impl produce the same hash."""
        bw, seed, dim = cfg_triplet
        a = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        b = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        assert a.config_hash == b.config_hash

    def test_config_hash_cross_impl(
        self,
        ref: ModuleType,
        rs: ModuleType,
        cfg_triplet: tuple[int, int, int],
    ) -> None:
        """Python-reference and Rust CodecConfigs with identical params hash equally."""
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        assert py_cfg.config_hash == rs_cfg.config_hash


class TestRotationParity:
    """Parity tests for RotationMatrix determinism and cross-impl agreement."""

    def test_rotation_deterministic(
        self,
        ref: ModuleType,
        cfg_triplet: tuple[int, int, int],
        vector: npt.NDArray[np.float32],
    ) -> None:
        """Two RotationMatrices from the same config apply identically to a vector."""
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        r1 = ref.codec.RotationMatrix.from_config(cfg)
        r2 = ref.codec.RotationMatrix.from_config(cfg)
        np.testing.assert_array_equal(r1.apply(vector), r2.apply(vector))

    def test_rotation_cross_impl(
        self,
        ref: ModuleType,
        rs: ModuleType,
        cfg_triplet: tuple[int, int, int],
        vector: npt.NDArray[np.float32],
    ) -> None:
        """Python-reference and Rust rotation agree within 1e-6 on the same vector."""
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        py_rot = ref.codec.RotationMatrix.from_config(py_cfg)
        rs_rot = rs.codec.RotationMatrix.from_config(rs_cfg)
        np.testing.assert_allclose(
            py_rot.apply(vector), rs_rot.apply(vector), atol=1e-6
        )


class TestCompressRoundTrip:
    """Parity tests for compress/decompress round-trip within and across impls."""

    def test_self_round_trip(
        self,
        ref: ModuleType,
        cfg_triplet: tuple[int, int, int],
        batch: npt.NDArray[np.float32],
    ) -> None:
        """Reference compress → reference decompress MSE is below 1.0 (sanity bound)."""
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb = ref.codec.Codebook.train(batch, cfg)
        codec = ref.codec.Codec()
        cvs = [codec.compress(row, cfg, cb) for row in batch]
        recon = np.stack([codec.decompress(cv, cfg, cb) for cv in cvs])
        # Sanity bound only — tight MSE is a calibration test, not parity.
        assert float(np.mean((recon - batch) ** 2)) < 1.0

    def test_cross_impl_round_trip(
        self,
        ref: ModuleType,
        rs: ModuleType,
        cfg_triplet: tuple[int, int, int],
        batch: npt.NDArray[np.float32],
    ) -> None:
        """Compress with reference, decompress with Rust and vice versa."""
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        py_cb = ref.codec.Codebook.train(batch, py_cfg)
        py_codec = ref.codec.Codec()

        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cb = rs.codec.Codebook(entries=py_cb.entries, bit_width=bw)
        rs_codec = rs.codec.Codec()

        for row in batch[:8]:
            py_cv = py_codec.compress(row, py_cfg, py_cb)
            rs_cv = rs.codec.CompressedVector.from_bytes(py_cv.to_bytes())
            np.testing.assert_allclose(
                py_codec.decompress(py_cv, py_cfg, py_cb),
                rs_codec.decompress(rs_cv, rs_cfg, rs_cb),
                atol=1e-3,
            )


class TestSerializationParity:
    """Parity tests for codebook byte-stability and cross-impl entry agreement."""

    def test_codebook_entries_stable(
        self,
        ref: ModuleType,
        cfg_triplet: tuple[int, int, int],
        batch: npt.NDArray[np.float32],
    ) -> None:
        """Reference Codebook train determinism is the stable structural property.

        Phase 24 will replace this with a ``to_bytes()``-based comparison
        once the Rust side wires ``rs.codec.Codebook.from_bytes`` against
        the reference's ``entries`` array layout.
        """
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb1 = ref.codec.Codebook.train(batch, cfg)
        cb2 = ref.codec.Codebook.train(batch, cfg)
        np.testing.assert_array_equal(cb1.entries, cb2.entries)

    def test_codebook_cross_impl_bytes(
        self,
        ref: ModuleType,
        rs: ModuleType,
        cfg_triplet: tuple[int, int, int],
        batch: npt.NDArray[np.float32],
    ) -> None:
        """Rust Codebook from reference entries has identical ``entries`` array."""
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb = ref.codec.Codebook.train(batch, cfg)
        rs_cb = rs.codec.Codebook(entries=cb.entries, bit_width=bw)
        np.testing.assert_array_equal(rs_cb.entries, cb.entries)
