"""Cross-implementation parity scaffold.

Phase 23 deliverable: structure exists, runs green against the
reference alone (trivial self-parity), and Phase 24 flips the
``rs`` fixture on to make every test meaningful.
"""
from __future__ import annotations

import numpy as np
import pytest


pytestmark = pytest.mark.parity


class TestConfigParity:
    def test_config_hash_matches_self(self, ref, cfg_triplet):
        bw, seed, dim = cfg_triplet
        a = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        b = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        assert a.config_hash == b.config_hash

    def test_config_hash_cross_impl(self, ref, rs, cfg_triplet):
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        assert py_cfg.config_hash == rs_cfg.config_hash


class TestRotationParity:
    def test_rotation_deterministic(self, ref, cfg_triplet, vector):
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        r1 = ref.codec.RotationMatrix.from_config(cfg)
        r2 = ref.codec.RotationMatrix.from_config(cfg)
        np.testing.assert_array_equal(r1.apply(vector), r2.apply(vector))

    def test_rotation_cross_impl(self, ref, rs, cfg_triplet, vector):
        bw, seed, dim = cfg_triplet
        py_cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        py_rot = ref.codec.RotationMatrix.from_config(py_cfg)
        rs_rot = rs.codec.RotationMatrix.from_config(rs_cfg)
        np.testing.assert_allclose(
            py_rot.apply(vector), rs_rot.apply(vector), atol=1e-6
        )


class TestCompressRoundTrip:
    def test_self_round_trip(self, ref, cfg_triplet, batch):
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb = ref.codec.Codebook.train(batch, cfg)
        codec = ref.codec.Codec()
        cvs = [codec.compress(row, cfg, cb) for row in batch]
        recon = np.stack([codec.decompress(cv, cfg, cb) for cv in cvs])
        # Sanity bound only — tight MSE is a calibration test, not parity.
        assert float(np.mean((recon - batch) ** 2)) < 1.0

    def test_cross_impl_round_trip(self, ref, rs, cfg_triplet, batch):
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
    def test_codebook_entries_stable(self, ref, cfg_triplet, batch):
        """Reference Codebook has no `to_bytes`; train determinism is the
        stable structural property we can assert on the reference alone.
        Phase 24 will replace this with a `to_bytes()`-based comparison
        once the Rust side wires `rs.codec.Codebook.from_bytes` against
        the reference's `entries` array layout."""
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb1 = ref.codec.Codebook.train(batch, cfg)
        cb2 = ref.codec.Codebook.train(batch, cfg)
        np.testing.assert_array_equal(cb1.entries, cb2.entries)

    def test_codebook_cross_impl_bytes(self, ref, rs, cfg_triplet, batch):
        bw, seed, dim = cfg_triplet
        cfg = ref.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        cb = ref.codec.Codebook.train(batch, cfg)
        rs_cb = rs.codec.Codebook(entries=cb.entries, bit_width=bw)
        np.testing.assert_array_equal(rs_cb.entries, cb.entries)
