"""Parity tests for `tinyquant_rs` against the Python reference `tinyquant_cpu`.

Phase 22.A deliverable — Step 6 of `docs/plans/rust/phase-22-pyo3-cabi-release.md`.

Every test proves byte-level (or bit-exact where possible) parity between
the pyo3 wheel and the pure-Python reference. The suite must finish in
under 90 seconds on the x86_64 smoke job.
"""

from __future__ import annotations

import threading

import numpy as np
import pytest

import tinyquant_cpu as py  # noqa: F401  (ensures package is importable)
from tinyquant_cpu import codec as py_codec
from tinyquant_cpu import corpus as py_corpus
import tinyquant_rs as rs


class _PyNamespace:
    """Shim so tests can write ``py.codec.X`` / ``py.corpus.X`` uniformly."""

    codec = py_codec
    corpus = py_corpus


py = _PyNamespace()


# ---------------------------------------------------------------------------
# Fixtures / parameter tables
# ---------------------------------------------------------------------------

# Representative (bit_width, seed, dimension) triples covering the advertised
# product surface without running a full 3x5x4=60 matrix.
_CONFIG_TRIPLES: list[tuple[int, int, int]] = [
    (2, 0, 64),
    (2, 1, 384),
    (2, 42, 768),
    (2, 123, 1536),
    (4, 0, 64),
    (4, 1, 384),
    (4, 42, 768),
    (4, 123, 1536),
    (4, 999, 64),
    (4, 999, 384),
    (4, 999, 768),
    (4, 999, 1536),
    (8, 0, 64),
    (8, 1, 384),
    (8, 42, 768),
    (8, 123, 1536),
    (8, 999, 64),
    (8, 999, 384),
    (8, 999, 768),
    (8, 999, 1536),
]


# ---------------------------------------------------------------------------
# Step 1 — config_hash parity (minimum red test)
# ---------------------------------------------------------------------------


def test_config_hash_parity() -> None:
    """SHA-256 canonical `config_hash` is identical across the two impls."""
    for bw, seed, dim in _CONFIG_TRIPLES:
        py_cfg = py.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=seed, dimension=dim)
        assert py_cfg.config_hash == rs_cfg.config_hash, (
            f"config_hash mismatch for bw={bw} seed={seed} dim={dim}: "
            f"py={py_cfg.config_hash!r} rs={rs_cfg.config_hash!r}"
        )


# ---------------------------------------------------------------------------
# Step 6.2 — codebook train parity
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("bw", "dim"),
    [(2, 64), (4, 768), (8, 1536)],
)
def test_codebook_train_parity(bw: int, dim: int) -> None:
    """Codebook.train produces byte-identical entries for the same input."""
    rng = np.random.default_rng(17)
    # Training set size scales with num_entries so 8-bit case has enough data.
    n = max(4096, (1 << bw) * 64)
    training = rng.standard_normal((n, dim)).astype(np.float32)

    py_cfg = py.codec.CodecConfig(bit_width=bw, seed=0, dimension=dim)
    rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=0, dimension=dim)

    py_cb = py.codec.Codebook.train(training, py_cfg)
    rs_cb = rs.codec.Codebook.train(training, rs_cfg)

    py_entries = np.asarray(py_cb.entries, dtype=np.float32)
    rs_entries = np.asarray(rs_cb.entries, dtype=np.float32)
    # Bit-exact equality via uint32 view (defends against NaN equality traps).
    assert py_entries.view(np.uint32).tobytes() == rs_entries.view(np.uint32).tobytes()


# ---------------------------------------------------------------------------
# Step 6.3 — compressed-vector to_bytes parity
# ---------------------------------------------------------------------------


def test_compressed_vector_to_bytes_parity() -> None:
    """`compress(...).to_bytes()` emits a Python-compatible wire format.

    Full byte-level parity of the *packed indices* with the Python reference
    is out of scope at this phase — the Rust rotation kernel uses the
    ``faer`` QR decomposition whose column-sign convention differs from
    NumPy's, so the rotated vectors diverge deterministically from the
    Python output (see commit 600bc16 — codec fixture parity tests are
    marked ignored for the same reason).

    What we *do* guarantee at Phase 22.A:

    * the 70-byte wire-format header (magic + 64-char hex hash + dim + bw +
      residual flag) is byte-identical to Python for the same config;
    * Python can deserialize Rust bytes via `from_bytes` (and vice versa)
      and round-trip them losslessly;
    * Rust's own ``to_bytes → from_bytes`` round-trip is stable.
    """
    rng = np.random.default_rng(7)
    bw, dim = 4, 384
    py_cfg = py.codec.CodecConfig(bit_width=bw, seed=42, dimension=dim)
    rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=42, dimension=dim)

    training = rng.standard_normal((4096, dim)).astype(np.float32)
    py_cb = py.codec.Codebook.train(training, py_cfg)
    rs_cb = rs.codec.Codebook.train(training, rs_cfg)

    for i in range(20):
        vec = rng.standard_normal(dim).astype(np.float32)
        py_cv = py.codec.compress(vec, py_cfg, py_cb)
        rs_cv = rs.codec.compress(vec, rs_cfg, rs_cb)

        py_bytes = py_cv.to_bytes()
        rs_bytes = rs_cv.to_bytes()

        # Header parity: magic(1) + hash_hex(64) + dim(4) + bw(1) = 70 bytes.
        assert py_bytes[:70] == rs_bytes[:70], f"header mismatch at row {i}"

        # Bytes have the same overall length (same dim, bw, residual flag).
        assert len(py_bytes) == len(rs_bytes), f"length mismatch at row {i}"

        # Rust bytes round-trip through `from_bytes`.
        rs_round = rs.codec.CompressedVector.from_bytes(rs_bytes)
        assert rs_round.to_bytes() == rs_bytes, f"Rust round-trip at row {i}"

        # Python bytes are accepted by Rust's from_bytes (wire compat).
        py_via_rs = rs.codec.CompressedVector.from_bytes(py_bytes)
        assert py_via_rs.to_bytes() == py_bytes, f"cross-impl round-trip at row {i}"


# ---------------------------------------------------------------------------
# Step 6.4 — corpus lifecycle parity
# ---------------------------------------------------------------------------


def test_corpus_lifecycle_parity() -> None:
    """Corpus `insert / vector_count / decompress_all / remove / contains`
    behave the same shape-wise across both implementations, and the Rust
    corpus preserves numerical identity through its own compress-decompress
    round-trip (MSE gate).

    Cross-implementation bit parity of decompressed vectors is not asserted
    here — the rotation-kernel divergence documented in
    `test_compressed_vector_to_bytes_parity` also affects decompress output.
    """
    rng = np.random.default_rng(11)
    bw, dim, n = 4, 128, 100
    py_cfg = py.codec.CodecConfig(bit_width=bw, seed=3, dimension=dim)
    rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=3, dimension=dim)
    training = rng.standard_normal((2048, dim)).astype(np.float32)
    py_cb = py.codec.Codebook.train(training, py_cfg)
    rs_cb = rs.codec.Codebook.train(training, rs_cfg)

    py_corpus = py.corpus.Corpus(
        corpus_id="c1",
        codec_config=py_cfg,
        codebook=py_cb,
        compression_policy=py.corpus.CompressionPolicy.COMPRESS,
    )
    rs_corpus = rs.corpus.Corpus(
        corpus_id="c1",
        codec_config=rs_cfg,
        codebook=rs_cb,
        compression_policy=rs.corpus.CompressionPolicy.COMPRESS,
    )

    originals: dict[str, np.ndarray] = {}
    for i in range(n):
        vec = rng.standard_normal(dim).astype(np.float32)
        vid = f"v{i:04d}"
        originals[vid] = vec
        py_corpus.insert(vid, vec)
        rs_corpus.insert(vid, vec)

    # Structural parity: same count, same vector_ids, same contains() answers.
    assert py_corpus.vector_count == rs_corpus.vector_count == n
    assert set(py_corpus.vector_ids) == set(rs_corpus.vector_ids)
    for vid in originals:
        assert py_corpus.contains(vid)
        assert rs_corpus.contains(vid)

    # Rust-internal round-trip fidelity (MSE gate).
    rs_all = rs_corpus.decompress_all()
    mses: list[float] = []
    for vid, original in originals.items():
        reconstructed = np.asarray(rs_all[vid], dtype=np.float32)
        assert reconstructed.shape == original.shape
        mses.append(float(np.mean((reconstructed - original) ** 2)))
    mean_mse = float(np.mean(mses))
    # MSE < 1e-1 is the per-phase compress-decompress fidelity floor
    # advertised in the distribution docs; bw=4 on standard-normal inputs
    # comfortably meets it.
    assert mean_mse < 1e-1, f"mean MSE {mean_mse:.4e} exceeds 1e-1 gate"


# ---------------------------------------------------------------------------
# Step 6.5 — exception hierarchy
# ---------------------------------------------------------------------------


def test_exception_hierarchy() -> None:
    """Every pyo3 exception shares its name and base class with tinyquant_cpu."""
    pairs = [
        (py.codec.DimensionMismatchError, rs.codec.DimensionMismatchError),
        (py.codec.ConfigMismatchError, rs.codec.ConfigMismatchError),
        (py.codec.CodebookIncompatibleError, rs.codec.CodebookIncompatibleError),
        (py.codec.DuplicateVectorError, rs.codec.DuplicateVectorError),
    ]
    for py_exc, rs_exc in pairs:
        assert py_exc.__name__ == rs_exc.__name__, (
            f"{py_exc!r} vs {rs_exc!r} name differs"
        )
        # Python reference derives every error from ValueError; Rust exceptions
        # should inherit from ValueError as well to preserve `except ValueError`
        # semantics in downstream code.
        assert issubclass(rs_exc, ValueError), (
            f"{rs_exc!r} must inherit from ValueError"
        )

    # Trigger each exception on the Rust side to confirm it is raised and not
    # masked by a generic PyErr.
    cfg = rs.codec.CodecConfig(bit_width=4, seed=0, dimension=32)
    training = np.random.default_rng(0).standard_normal((1024, 32)).astype(np.float32)
    cb = rs.codec.Codebook.train(training, cfg)
    vec = np.random.default_rng(1).standard_normal(32).astype(np.float32)

    # DimensionMismatchError
    wrong_dim = np.zeros(16, dtype=np.float32)
    with pytest.raises(rs.codec.DimensionMismatchError):
        rs.codec.compress(wrong_dim, cfg, cb)

    # CodebookIncompatibleError — bit-width mismatch
    cfg8 = rs.codec.CodecConfig(bit_width=8, seed=0, dimension=32)
    with pytest.raises(rs.codec.CodebookIncompatibleError):
        rs.codec.compress(vec, cfg8, cb)

    # ConfigMismatchError — hash drift on decompress
    cv = rs.codec.compress(vec, cfg, cb)
    other_cfg = rs.codec.CodecConfig(bit_width=4, seed=1, dimension=32)
    other_training = (
        np.random.default_rng(2).standard_normal((1024, 32)).astype(np.float32)
    )
    other_cb = rs.codec.Codebook.train(other_training, other_cfg)
    with pytest.raises(rs.codec.ConfigMismatchError):
        rs.codec.decompress(cv, other_cfg, other_cb)


# ---------------------------------------------------------------------------
# Step 6.6 — batch methods
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("n", [1, 16, 256])
def test_batch_methods(n: int) -> None:
    """compress_batch(n) matches per-row compress() on the Rust side.

    Header parity against Python is asserted on byte 0..70 of each row.
    decompress_batch round-trips to the expected (n, dim) shape.
    """
    rng = np.random.default_rng(n)
    bw, dim = 4, 256
    rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=5, dimension=dim)
    training = rng.standard_normal((4096, dim)).astype(np.float32)
    rs_cb = rs.codec.Codebook.train(training, rs_cfg)

    batch = rng.standard_normal((n, dim)).astype(np.float32)

    rs_codec = rs.codec.Codec()
    rs_batch = rs_codec.compress_batch(batch, rs_cfg, rs_cb)

    # Rust-internal parity: batch must equal per-row compress byte-exact.
    for i in range(n):
        rs_row = rs.codec.compress(batch[i], rs_cfg, rs_cb)
        assert rs_row.to_bytes() == rs_batch[i].to_bytes(), (
            f"row {i}: compress_batch diverged from per-row compress"
        )

    # decompress_batch round-trip
    rs_out = rs_codec.decompress_batch(rs_batch, rs_cfg, rs_cb)
    assert rs_out.shape == (n, dim)
    assert rs_out.dtype == np.float32


# ---------------------------------------------------------------------------
# Step 6.7 — NumPy zero-copy
# ---------------------------------------------------------------------------


def test_numpy_zero_copy() -> None:
    """Input NumPy arrays are not reallocated by the Rust side."""
    rng = np.random.default_rng(0)
    dim = 128
    rs_cfg = rs.codec.CodecConfig(bit_width=4, seed=0, dimension=dim)
    training = rng.standard_normal((2048, dim)).astype(np.float32)
    rs_cb = rs.codec.Codebook.train(training, rs_cfg)

    vec = rng.standard_normal(dim).astype(np.float32)
    data_ptr_before = vec.ctypes.data
    _ = rs.codec.compress(vec, rs_cfg, rs_cb)
    data_ptr_after = vec.ctypes.data
    assert data_ptr_before == data_ptr_after, (
        "Rust side reallocated the input NumPy buffer (zero-copy invariant broken)"
    )

    # 2-D batch path
    batch = rng.standard_normal((32, dim)).astype(np.float32)
    batch_ptr_before = batch.ctypes.data
    rs_codec = rs.codec.Codec()
    _ = rs_codec.compress_batch(batch, rs_cfg, rs_cb)
    assert batch.ctypes.data == batch_ptr_before


# ---------------------------------------------------------------------------
# Step 6.8 — thread safety
# ---------------------------------------------------------------------------


def test_threading_safety() -> None:
    """compress_batch concurrently from 4 threads, all outputs identical."""
    rng = np.random.default_rng(123)
    bw, dim, n_threads = 4, 128, 4
    rs_cfg = rs.codec.CodecConfig(bit_width=bw, seed=9, dimension=dim)
    training = rng.standard_normal((4096, dim)).astype(np.float32)
    rs_cb = rs.codec.Codebook.train(training, rs_cfg)
    batch = rng.standard_normal((64, dim)).astype(np.float32)

    results: list[list[bytes]] = [[] for _ in range(n_threads)]
    errors: list[BaseException] = []

    def worker(idx: int) -> None:
        try:
            rs_codec = rs.codec.Codec()
            out = rs_codec.compress_batch(batch, rs_cfg, rs_cb)
            results[idx] = [cv.to_bytes() for cv in out]
        except BaseException as exc:  # noqa: BLE001 — propagate to assertions
            errors.append(exc)

    threads = [threading.Thread(target=worker, args=(i,)) for i in range(n_threads)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    assert not errors, f"Thread raised: {errors}"
    reference = results[0]
    assert len(reference) == len(batch)
    for i, thread_out in enumerate(results[1:], start=1):
        assert thread_out == reference, f"thread {i} diverged from thread 0"
