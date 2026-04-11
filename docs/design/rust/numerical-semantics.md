---
title: Rust Port — Numerical Semantics and Determinism
tags:
  - design
  - rust
  - determinism
  - numerical
  - parity
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Numerical Semantics and Determinism

> [!info] Purpose
> Document exactly how the Rust port reproduces Python's numerical
> behavior, where bit-for-bit parity is achievable, where it is not,
> and what the parity gate measures instead in the latter case.

## Parity definitions (in decreasing strictness)

| Level | Definition | Applies to |
|---|---|---|
| **Byte parity** | Identical raw bytes for the same inputs across implementations | `CompressedVector::to_bytes`, `CodecConfig::config_hash`, bit-packed indices, residual payload |
| **Bit parity** | Identical `f32` bit patterns | Rotation *output* after forward rotate→inverse rotate, codebook entries after training on fixed input |
| **Tight numerical parity** | `max(abs(rust - python)) < 1e-6` on unit vectors | Single-rotation output |
| **Score fidelity parity** | `|cos_sim_rust - cos_sim_python| < 1e-5` | Round-trip compressed→decompressed cosine similarity |
| **Statistical parity** | Pearson ρ between pairwise similarities ≥ 0.9999 | Full-pipeline batch output |

We aim for **byte parity** on everything except the rotation matrix
generation, and **tight numerical parity** (or better) on the rotation
itself — see the analysis below.

## Rotation matrix — the hardest parity question

Python uses NumPy's `default_rng(seed)` which is PCG64 backed by a
SIMD-accelerated Ziggurat for `standard_normal`. Pure Rust crates
(`rand_pcg`, `rand_distr`) implement PCG64 and Ziggurat but the
resulting sample streams are **not** guaranteed bit-identical to
NumPy's because:

1. NumPy's `default_rng.standard_normal` uses a platform-specific
   SIMD path in NumPy ≥ 1.22 on x86_64. The canonical NumPy-tested
   reference values are only guaranteed inside NumPy.
2. NumPy's Ziggurat table and ordering choices differ slightly from
   `rand_distr::StandardNormal`, even with identical PCG64 states.
3. `np.linalg.qr` (LAPACK `dgeqrf`) and `faer`'s Householder QR can
   disagree on the sign convention of `R` in the absence of pivoting,
   even though both produce a valid orthogonal `Q`.

Therefore **bit-identical rotation matrices are a non-goal**. The
design instead pins parity at the *effect* level:

### The three-step determinism plan for rotations

**Step 1 — Canonical seed → canonical f64 standard-normal stream.**

We ship a small pure-Rust implementation called
`ChaChaGaussianStream` that:

- Uses `rand_chacha::ChaCha20Rng` seeded from the `u64` seed via
  `ChaCha20Rng::seed_from_u64(seed)`.
- Draws uniform f64 values and applies the Box-Muller transform to
  produce pairs of standard-normal f64 samples.

This stream is implementation-defined by *us*, not by NumPy. It is
stable across platforms and Rust versions because `rand_chacha` is
pinned and `f64::from_bits` is IEEE-754 deterministic.

**Step 2 — Canonical f64 stream → canonical orthogonal matrix.**

The stream fills a row-major `dim × dim` f64 matrix. We then compute a
Householder QR via `faer::linalg::qr::no_pivoting::compute`, take `Q`,
and apply the Haar-measure sign correction: for each column `j`,
multiply by `sign(R[j, j])`. Result is `Q_canonical`.

**Step 3 — Parity bridge to Python.**

`tinyquant-py` ships a small helper
`_install_canonical_rotation(seed, dim)` that, when present, overrides
Python's `RotationMatrix._cached_build` to use the same
`ChaChaGaussianStream` path (called through the pyo3 binding or a
vendored pure-NumPy reimplementation in Python). Two modes ship:

| Mode | Python uses | Rust uses | Parity |
|---|---|---|---|
| `legacy` (default for Python 0.1.1) | NumPy PCG64 + LAPACK QR | Canonical ChaCha + faer QR | Statistical — rotation effects agree to Pearson ρ > 0.9999 |
| `canonical` (new in Python 0.2.0) | ChaCha reference via pyo3 or pure Python fallback | Same | **Bit parity** on the matrix f64 bytes |

**Which mode we ship for the Rust 0.1.0 phase**

We ship the `canonical` mode *and* a `legacy` compatibility layer that
replays a pre-captured set of Python-generated matrices for the
specific `(seed, dimension)` pairs used by the gold calibration
corpus. These are stored as fixture files under
`rust/crates/tinyquant-core/tests/fixtures/rotation/seed_{seed}_dim_{dim}.f64.bin`.
During tests the Rust code loads them and asserts parity against its
own canonical output within 1e-12 (they are independent of Python
once captured).

This is the minimal invasive path: no Python runtime modification,
calibration results in both implementations are identical on the gold
fixtures, and any downstream consumer that needs perfect Python
parity for a new `(seed, dimension)` can call `xtask fixtures refresh`
to capture a new f64 matrix from Python.

> [!warning] Cross-platform faer QR nondeterminism (R19)
> Phase 14's PR CI surfaced that `faer::Mat::qr()` at `dim=768`
> does **not** produce bit-identical output across CI runners.
> The Windows-generated `seed_42_dim_768.f64.bin` fixture disagrees
> with Linux CI runs by ~90% of its f64 words even under
> `RAYON_NUM_THREADS=1`, because `pulp` picks different SIMD
> kernels (AVX2 vs AVX-512) per host CPU at runtime. The `dim=64`
> fixture still matches because it falls below all of faer's
> dispatch thresholds.
>
> **Current state (Phase 14).** The bit-exact test is
> `#[ignore]`d with an inline comment pointing at R19, and a new
> `seed_42_dim_768_build_is_orthogonal_within_1e_12` test checks
> the mathematical invariant (`QᵀQ = I`) instead of a byte-for-byte
> fingerprint. Orthogonality is portable across SIMD kernels and
> thread counts.
>
> **Long-term fix.** Thread an explicit `faer::Parallelism::None`
> through `RotationMatrix::build`
> (`rust/crates/tinyquant-core/src/codec/rotation_matrix.rs:78`)
> **and** force a scalar (non-SIMD) QR path so pulp's runtime
> dispatch is bypassed. Once that lands, regenerate the fixture
> once from the serial scalar kernel and re-enable the bit-exact
> test with a regeneration-matrix test that varies
> `RAYON_NUM_THREADS` across `{1, 2, nproc}`.
>
> Any future "Rust-canonical" fixture that claims bit-exact
> cross-platform parity must ship with the determinism contract
> in code and must be validated on at least two CI runners with
> distinct host CPUs. A workflow env var is not a substitute —
> see
> [[design/rust/risks-and-mitigations#r19-faer-kernel-nondeterminism-across-platforms-and-cpus|Risks §R19]]
> and [[design/rust/phase-14-implementation-notes|Phase 14
> Implementation Notes]] §L4 for root-cause and remediation notes.

## Quantization — byte parity achievable

Python:

```python
idx = np.searchsorted(entries, values, side="left")
idx = np.clip(idx, 0, num_entries - 1)
left_idx = np.clip(idx - 1, 0, num_entries - 1)
left_dist = np.abs(values - entries[left_idx])
right_dist = np.abs(values - entries[idx])
result = np.where(left_dist < right_dist, left_idx, idx)
```

Rust replicates the same control flow element by element. Because the
f32 subtraction and `abs` are IEEE-754 deterministic, and the
tie-breaking (`left_dist < right_dist` — strict less-than — favors the
right neighbor on equality, same as NumPy), the output is bit
identical.

Phase 14 confirmed this in code: `tinyquant_core::codec::quantize`
holds a `pub(crate)` `scalar_quantize` kernel that walks `entries` with
`iter::position(|e| !(e < v))` (equivalent to
`np.searchsorted(side="left")`), clips to `[0, n-1]`, and compares
`left_dist < right_dist` with a strict `<`. `Codebook::quantize_into`
and `Codebook::dequantize_into` delegate to that kernel, so the Phase
20 SIMD path has a single panic-free reference to diff against.

Parity gate: 10 000 random f32 values from
`numpy.random.default_rng(7).standard_normal(10_000).astype(float32)`
produce 10 000 bytes of expected indices per bit width. The
`expected_bw{2,4,8}_seed42.u8.bin` fixtures capture those 30 000
bytes total, and `tests/quantize.rs` asserts
`cb.quantize_into(values) == fixture_bytes` bit-for-bit on every
supported bit width. See [[design/rust/phase-14-implementation-notes|
Phase 14 Implementation Notes]] for the lint-wall workarounds that
kept the kernel panic-free under the crate's
`clippy::pedantic + nursery + unwrap_used + indexing_slicing` profile.

## Dequantization — trivial parity

Both implementations implement `entries[indices[i]]` which is a gather
with no floating-point arithmetic. Parity is trivial; the only thing
to assert is that `indices` are `u8` in both paths.

## Codebook training — byte parity with care

Python:

```python
flat = vectors.flatten().astype(np.float64)
quantiles = np.linspace(0, 1, num_entries)
entries = np.quantile(flat, quantiles).astype(np.float32)
entries = np.sort(entries)
```

The subtle point is `np.quantile` default interpolation
(`method="linear"`). The formula is:

```
h = (N - 1) * q                                    (rank)
i = floor(h)
frac = h - i
value = flat_sorted[i] + frac * (flat_sorted[i+1] - flat_sorted[i])
```

Rust implementation in `tinyquant-core::codec::codebook` — Phase 14
confirmed byte parity on the full training path:

1. Sort the flattened f64 buffer in-place (`Vec<f64>::sort_by` with
   `f64::total_cmp` to match NumPy's behavior on NaNs — the gold
   fixtures contain no NaNs, but `total_cmp` is the right default).
2. For each `q_k = k / (num_entries - 1)`, compute `h`, `i`, `frac` as
   above (`libm::floor` instead of `std::f64::floor` so the module
   stays `no_std`-compatible).
3. Interpolate in f64, then cast to f32 via `as f32` (which uses
   round-to-nearest-even).
4. Sort the resulting f32 array with `f32::total_cmp` as a defensive
   no-op. Any tie then fails with
   `CodecError::InsufficientTrainingData { expected }` *before* the
   value reaches `Codebook::new`, so the insufficient-data story is
   surfaced with the most informative error.
5. Hand the entries off to `Codebook::new` so the invariant checks
   run a second time through the single source-of-truth constructor.

Because both NumPy and our code use IEEE-754 f64 arithmetic with the
same sequence of operations, the output is bit identical *for inputs
that do not suffer catastrophic cancellation* — which applies to all
embedding-scale data. Phase 14 locked this in by freezing three
fixture files, one per bit width, at
`rust/crates/tinyquant-core/tests/fixtures/codebook/expected_bw{2,4,8}_seed42.f32.bin`,
plus the shared 10 000 × 64 training corpus from
`numpy.random.default_rng(42).standard_normal`. The
`train_matches_python_fixture_bw{2,4,8}_seed42_n10000_d64`
integration tests compare bit-for-bit using `f32::to_bits`, which
would catch a 1-ULP drift if any future dep bump introduced one.
Refresh goes through `cargo xtask fixtures refresh-codebook`, which
must be followed by `refresh-quantize` because the quantize indices
are pinned to the matching codebook bytes.

## Residual — fp16 parity

Python:

```python
diff = original - reconstructed                # f32 - f32 → f32
bytes = diff.astype(np.float16).tobytes()      # f32 → f16, little-endian
```

Rust uses the `half` crate for `f16`:

```rust
use half::f16;

let mut out = Vec::with_capacity(diff.len() * 2);
for &d in diff.iter() {
    out.extend_from_slice(&f16::from_f32(d).to_le_bytes());
}
```

`f16::from_f32` uses round-to-nearest-even, matching NumPy's
`astype(np.float16)`. Parity tested on 1 million random f32 pairs.

## Rotation application — f64 intermediate is required

Python:

```python
return (self.matrix @ vector.astype(np.float64)).astype(np.float32)
```

Rust:

```rust
// 1. Extend input to f64.
// 2. Matrix-vector multiply in f64 (BLAS DGEMV or faer).
// 3. Cast to f32.
```

The matrix-vector multiply order matters: NumPy uses column-major
accumulation on row-major arrays via BLAS, while faer uses a different
kernel. Floating-point addition is not associative, so the exact f64
bit pattern can differ by up to 1 ULP between implementations. The
f32 cast absorbs this: both implementations produce byte-identical f32
output in >99.99% of cases, and the remaining edge cases differ by at
most 1 f32 ULP, which does not affect downstream cosine similarity.

**What the parity test asserts**: for 10 000 random vectors and 3
supported dimensions, `max_abs_diff < 1e-5` and average `|diff|` < 1e-7.
A weaker `max < 1e-4` is acceptable on a single vector; anything
beyond that is a bug.

## fp16 round-trip — tight numerical parity

The residual introduces a precision ceiling because `f16` has only 10
explicit mantissa bits. For embedding values typically in `[-2, 2]`,
this bounds the residual error at ~1e-3 per coordinate.
**This is not a parity issue** — it's a fundamental property of the
format — but the test suite needs to know about it when building
tolerance envelopes for round-trip fidelity.

## Hash parity — byte parity is mandatory

The `config_hash` is consumed as a string identity check. Python:

```python
canonical = (
    f"CodecConfig("
    f"bit_width={self.bit_width},"
    f"seed={self.seed},"
    f"dimension={self.dimension},"
    f"residual_enabled={self.residual_enabled})"
)
return hashlib.sha256(canonical.encode(), usedforsecurity=False).hexdigest()
```

Rust must reproduce this string byte for byte. The tricky bits:

1. `residual_enabled` is Python's `str(bool)` — either `"True"` or
   `"False"` (capitalized). Rust must emit those literal strings, not
   `"true"`/`"false"`.
2. `seed` and `dimension` are Python's `str(int)` — no leading `+`,
   no thousands separators. Rust's `u64`/`u32` `Display` matches.
3. `encode()` defaults to UTF-8; every ASCII character maps to a
   single byte. Rust's `str::as_bytes()` is identical for ASCII.
4. `sha256(...).hexdigest()` is lowercase hex; Rust's `hex::encode` is
   lowercase by default.

Parity gate: for all combinations of
`bit_width ∈ {2, 4, 8}`, `seed ∈ {0, 1, 42, 999, u64::MAX}`,
`dimension ∈ {1, 32, 768, 1536}`,
`residual_enabled ∈ {true, false}` (120 total), the Rust hash equals
the Python hash byte for byte. Fixture file
`rust/crates/tinyquant-core/tests/fixtures/config_hashes.json`
captures the expected values and is regenerated only by
`xtask fixtures refresh` (with explicit human approval via commit).

## Serialization — byte parity is mandatory

Python header:

```python
_HEADER_FORMAT: str = "<B64sIB"  # version, hash, dimension, bit_width
_HEADER_SIZE: int = 71            # = 1 + 64 + 4 + 1 + 1 (struct padding)
```

Wait — `struct.calcsize("<B64sIB") == 71`. Let us verify: `<` no
alignment; `B` = 1; `64s` = 64; `I` = 4; `B` = 1. Total = 70. Python
reports 71 because of a trailing alignment byte? No — with `<` (no
padding), `struct.calcsize("<B64sIB") == 70`. Re-reading the Python
source shows the fields are exactly `version, hash_raw, dimension,
bit_width` and the header is 70 bytes by `struct.calcsize`.

**The Python source says 71 in a comment but the actual value
returned by `struct.calcsize("<B64sIB")` is 70.** The Rust
implementation uses **70** as the canonical header size and a parity
test asserts `len(to_bytes(cv)) - packed_indices_len - residual_tail == 70`.
If anyone ever changes Python's header format, the parity gate will
catch it on the next run.

> [!warning] Header-size audit
> During the Rust implementation phase, first write a parity test that
> computes `len(cv.to_bytes()) - ceil(dim*bit_width/8) - residual_overhead`
> on a known input and records the actual header size. Rust then
> follows that number. The comment in `compressed_vector.py` is
> informational only.

Rust's `tinyquant-io::compressed_vector::to_bytes` emits bytes in the
same order:

```
[0]         version (u8 = 0x01)
[1..65]     config_hash (64 bytes, UTF-8, null-padded if shorter)
[65..69]    dimension (u32 little-endian)
[69]        bit_width (u8)
[70..70+P]  packed indices (P = ceil(dim*bit_width/8))
[70+P]      residual flag (u8; 0x00 or 0x01)
[70+P+1..]  if flag: residual length (u32 LE) + residual bytes
```

The bit packing (LSB-first, cross-byte boundary handling) is a
byte-for-byte reimplementation of Python's `_pack_indices`. Parity is
verified on a corpus of 10 000 randomly-generated compressed vectors,
round-tripped through Python to produce fixture bytes, which Rust
reads and re-emits; bytes must match exactly.

## NaN and infinity handling

The Python implementation does not explicitly handle NaN or infinity
in vectors; it relies on NumPy's default behavior (silent propagation
through arithmetic). Rust mirrors this: no guard rails. A parity test
confirms that `compress(vector_with_nan) == compress(vector_with_nan)`
in both implementations at the level they agree on (both explode in
roughly the same place — the quantization searchsorted on NaN is
undefined but implementations agree on the *output*).

We add a single documented note in `CodecConfig::new` rustdoc saying
"NaN and ±inf inputs produce implementation-defined output; use
`is_finite()` guards at the application layer."

## Round-trip error bounds (what tests assert)

| Bit width | Residual | Max f32 MSE bound | Pearson ρ floor |
|---|---|---|---|
| 8 | off | < 1e-3 | > 0.999 |
| 8 | on | < 1e-4 | > 0.9999 |
| 4 | off | < 1e-1 | > 0.98 |
| 4 | on | < 1e-2 | > 0.995 |
| 2 | off | < 2.5e-1 | > 0.92 |
| 2 | on | < 1e-1 | > 0.95 |

These numbers come from calibration runs on the 10 000-vector gold
fixture. The Rust port must match them within the same tolerances.

## See also

- [[design/rust/crate-topology|Crate Topology]]
- [[design/rust/type-mapping|Type Mapping from Python]]
- [[design/rust/serialization-format|Serialization Format]]
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/risks-and-mitigations|Risks and Mitigations]]
