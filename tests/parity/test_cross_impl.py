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
        batch: npt.NDArray[np.float32],
    ) -> None:
        """Legacy mode: both impls produce valid orthogonal matrices.

        Tight numerical parity on raw rotation output is a non-goal in legacy
        mode: NumPy PCG64 and ChaCha20 produce different Gaussian samples so
        the matrices differ completely. The gate is the invariant both must
        satisfy — an orthogonal transform preserves pairwise cosine
        similarities exactly. We verify Pearson ρ ≥ 0.9999 between original
        and rotated pairwise similarities for each implementation.

        Bit-identical rotation matrices across impls require canonical mode;
        see Phase 28.7 plan.
        """
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        py_rot = ref.codec.RotationMatrix.from_config(py_cfg)
        rs_rot = rs.codec.RotationMatrix.from_config(rs_cfg)

        vecs = batch[:16].astype(np.float64)
        py_rotated = np.stack(
            [py_rot.apply(r.astype(np.float32)).astype(np.float64) for r in vecs]
        )
        rs_rotated = np.stack(
            [rs_rot.apply(r.astype(np.float32)).astype(np.float64) for r in vecs]
        )

        def _pairwise_cos_sims(
            m: npt.NDArray[np.float64],
        ) -> npt.NDArray[np.float64]:
            unit = m / np.linalg.norm(m, axis=1, keepdims=True)
            gram = unit @ unit.T
            return gram[np.triu_indices(len(m), k=1)]

        orig_sims = _pairwise_cos_sims(vecs)
        py_sims = _pairwise_cos_sims(py_rotated)
        rs_sims = _pairwise_cos_sims(rs_rotated)

        rho_py = float(np.corrcoef(orig_sims, py_sims)[0, 1])
        rho_rs = float(np.corrcoef(orig_sims, rs_sims)[0, 1])
        assert rho_py >= 0.9999, (
            f"Python RotationMatrix breaks cosine-similarity preservation: "
            f"ρ={rho_py:.6f}"
        )
        assert rho_rs >= 0.9999, (
            f"Rust RotationMatrix breaks cosine-similarity preservation: "
            f"ρ={rho_rs:.6f}"
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
        """Legacy mode: byte serialization is interoperable; same-impl MSE within sanity bound.

        Cross-impl decompression parity (py-compress → rs-decompress giving
        the same output as py-decompress) requires canonical mode: Python
        compresses in PCG64-rotation space while Rust decompresses using
        ChaCha20 inverse rotation, so reconstructions diverge completely.
        That gate is a Phase 28.7 deliverable.

        What we assert here without canonical mode:
        - Python-compressed bytes round-trip through the Rust serializer
          unchanged (`CompressedVector.from_bytes` + `.to_bytes()` is stable).
        - Python same-impl round-trip MSE is within the sanity bound (< 1.0).
        """
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        py_cb = ref.codec.Codebook.train(batch, py_cfg)
        py_codec = ref.codec.Codec()

        for row in batch[:8]:
            py_cv = py_codec.compress(row, py_cfg, py_cb)

            # Byte serialization compatibility: Rust can load Python-compressed
            # bytes and re-serialise them without corruption.
            rs_cv = rs.codec.CompressedVector.from_bytes(py_cv.to_bytes())
            assert rs_cv.to_bytes() == py_cv.to_bytes(), (
                "CompressedVector byte round-trip through Rust is not stable"
            )

            # Same-impl reconstruction quality.
            py_recon = py_codec.decompress(py_cv, py_cfg, py_cb)
            mse = float(
                np.mean(
                    (py_recon.astype(np.float64) - row.astype(np.float64)) ** 2
                )
            )
            assert mse < 1.0, (
                f"Python same-impl round-trip MSE {mse:.4f} exceeds sanity bound"
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
