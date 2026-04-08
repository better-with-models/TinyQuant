"""Stub calibration tests for research alignment — placeholder for Phase 10."""

import numpy as np

from tinyquant.codec import Codec, CodecConfig


def test_residual_correction_improves_fidelity() -> None:
    """Enabling residuals produces lower reconstruction error."""
    dim = 64
    config_no_res = CodecConfig(
        bit_width=4, dimension=dim, seed=7, residual_enabled=False
    )
    config_res = CodecConfig(bit_width=4, dimension=dim, seed=7, residual_enabled=True)
    codec = Codec()
    rng = np.random.default_rng(7)
    data = rng.standard_normal((30, dim)).astype(np.float32)

    codebook_no = codec.build_codebook(data, config_no_res)
    codebook_yes = codec.build_codebook(data, config_res)

    errors_no = []
    errors_yes = []
    for v in data[:10]:
        dec_no = codec.decompress(
            codec.compress(v, config_no_res, codebook_no), config_no_res, codebook_no
        )
        dec_yes = codec.decompress(
            codec.compress(v, config_res, codebook_yes), config_res, codebook_yes
        )
        errors_no.append(float(np.linalg.norm(v - dec_no)))
        errors_yes.append(float(np.linalg.norm(v - dec_yes)))

    mean_no = np.mean(errors_no)
    mean_yes = np.mean(errors_yes)
    assert mean_yes < mean_no, (
        f"Residual correction did not help: {mean_yes:.4f} >= {mean_no:.4f}"
    )
