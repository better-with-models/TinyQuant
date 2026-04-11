---
title: "Phase 13: Rotation Matrix and Numerics — Implementation Notes"
tags:
  - design
  - rust
  - phase-13
  - rotation
  - numerics
  - implementation-notes
  - retrospective
date-created: 2026-04-10
date-completed: 2026-04-10
status: stable
category: design
---

# Phase 13 — Implementation Notes

> [!info] Purpose
> Captures the design decisions, gotchas, and deviations from the original
> Phase 13 plan ([[plans/rust/phase-13-rotation-numerics|Phase 13 Plan]])
> that arose during execution on 2026-04-10. Future Rust phases should
> read this alongside [[design/rust/numerical-semantics|Numerical Semantics]]
> and [[design/rust/risks-and-mitigations|Risks and Mitigations]] to pick
> up invariants that are now locked-in by code and fixtures rather than by
> prose alone.

> [!note] Relationship to the plan doc
> [[plans/rust/phase-13-rotation-numerics|Phase 13 Plan]] describes the
> *intended* 17-step sequence. This page records the *actual* outcome:
> what landed, what was reshaped mid-execution, and why. The plan doc
> itself has been flipped to `status: complete` — treat it as the
> intent-of-record and this page as the execution log.

## What landed

Phase 13 shipped as five focused commits on `main`:

1. `chore(workspace)` — wire `libm`, `spin`, `serde_json`, and the `hex`
   `alloc` feature into the workspace.
2. `feat(tinyquant-core)` — add the `codec` module (`CodecConfig`,
   `ChaChaGaussianStream`, `RotationMatrix`, `RotationCache`) plus 29
   integration tests.
3. `feat(tinyquant-core)` — freeze two rotation fixtures under Git LFS
   and add the `dump_rotation_fixture` example binary plus the
   `scripts/generate_rust_fixtures.py` tool.
4. `chore(xtask)` — add `cargo xtask fixtures refresh-{hashes,rotation,all}`
   and fix the pre-existing broken `cargo xtask` alias.
5. `docs(rust)` — mark Phase 13 complete and append the `log.md` entry.

New public surface in `tinyquant-core`:

- `tinyquant_core::codec::CodecConfig` — primary value object.
- `tinyquant_core::codec::RotationMatrix` — row-major `Arc<[f64]>`.
- `tinyquant_core::codec::RotationCache` — 8-entry spin LRU.
- `tinyquant_core::codec::SUPPORTED_BIT_WIDTHS` (`&[2, 4, 8]`).
- `tinyquant_core::codec::ROTATION_CACHE_DEFAULT_CAPACITY` (`8`).
- `tinyquant_core::codec::gaussian::ChaChaGaussianStream` remains
  `pub(crate)` — downstream code should always go through
  `RotationMatrix::build`.

Prelude now re-exports `CodecConfig`, `RotationMatrix`, `RotationCache`,
`ROTATION_CACHE_DEFAULT_CAPACITY`, and `SUPPORTED_BIT_WIDTHS` alongside
the Phase 12 type aliases and error enums.

## Deviations from the original plan

### Rust-canonical fixtures (not Python-canonical)

The plan doc's Step 15 asks for effect-level parity between a
Python-generated rotation fixture and a freshly built Rust matrix,
bounded by `1e-5`. That target is unreachable *in principle* because:

- The Python reference uses `numpy.random.default_rng(seed).standard_normal`
  (PCG64 + Ziggurat).
- The canonical Rust path uses `ChaCha20Rng::seed_from_u64(seed)` plus
  polar Box-Muller.

Two different RNG streams produce two different orthogonal matrices.
Applied to the same vector they will not agree to `1e-5`; they differ by
`O(1)`. [[design/rust/risks-and-mitigations|Risks and Mitigations §R1]]
already acknowledged this, but the plan doc's own fixture step read as
if the two could be reconciled.

**Resolution.** Phase 13 anchors rotation parity against **Rust-owned
frozen snapshots** instead. The `dump_rotation_fixture` example writes
row-major little-endian f64 bytes from the Rust canonical pipeline; the
fixtures are then diffed bit-for-bit on every subsequent build by
`tests/rotation_fixture_parity.rs`. This turns the fixtures into a
regression gate for the ChaCha + faer QR + Haar-sign-correction pipeline
rather than a cross-language parity gate.

The cross-language parity story now has two distinct layers, and Phase 13
only promises the first:

| Layer | Phase | Status |
|---|---|---|
| Canonical Rust vs frozen Rust snapshot (bit parity on f64) | 13 | ✅ landed |
| `config_hash` byte parity vs Python `tinyquant_cpu.codec.CodecConfig` | 13 | ✅ landed (120-triple sweep) |
| Legacy Python PCG64 vs canonical Rust — cosine parity within `1e-5` | 15+ | deferred |
| pyo3 bridge to let Python consumers opt into the canonical path | 15+ | deferred |

The deferral is tracked under [[plans/rust/phase-15-codec-service-residual|Phase 15]]
scope and in `docs/log.md` on 2026-04-10.

### proptest dropped

The original plan wanted `proptest` as a dev-dependency. Adding it
failed on `rust 1.81.0`: `proptest` 1.5+ pulls in `getrandom 0.4.2`
transitively, which requires the still-unstable `edition2024` cargo
feature. Options considered:

1. Pin `proptest = "=1.4"` — still pulls newer `rand` / `getrandom`
   variants because its caret spec is honoured inside `rand`.
2. Bump MSRV to a rust that supports `edition2024` — hard no, Phase 12
   only just finished bumping 1.78 → 1.81 for `core::error::Error` +
   `thiserror` v2.
3. Drop `proptest` entirely.

We went with option 3. The planned proptest use case was the 120-triple
exhaustive `config_hash` sweep; a plain nested-loop unit test driven by
the JSON fixture covers the same ground with no randomness needed. The
sweep is in `tests/codec_config.rs::config_hash_matches_python_all_120_triples`.

If a future phase really needs `proptest`, it will come in with a MSRV
bump and its own justification.

### `libm` added (not in original plan)

`tinyquant-core` is `#![no_std]`. The core `f64` operations used by the
Box-Muller pipeline (`ln`, `sqrt`, `sin`, `cos`, `abs`) are **std-only**
inherent methods on `f64`. In `no_std` we need an explicit floating-point
math crate. `libm` is the canonical answer: it is `no_std`-first, already
in the dependency graph via `faer`, and its pure-Rust implementation
plays nicely with the `thumbv7em-none-eabihf` target.

Added as a workspace dep and imported in `gaussian.rs` (for `sqrt`,
`log`, `sin`, `cos`) and `rotation_matrix.rs` (for `fabs`).

### `dump_rotation_fixture` as a `[[example]]`, not a CLI

The plan left open whether fixture generation should live in a new
`tinyquant-fixturegen/` crate, an `xtask` subroutine, or an example under
`tinyquant-core`. We picked the example:

- Zero new crates; no Cargo topology churn.
- `required-features = ["std"]` keeps the no_std build clean —
  `cargo build --no-default-features` skips it, `cargo check --all-targets`
  under default still exercises it.
- `cargo xtask fixtures refresh-rotation` delegates to it, so the
  end-user UX is unchanged.

## Gotchas worth remembering

### Python bool stringification is `"True"` / `"False"`, capitalised

`CodecConfig.config_hash` is a SHA-256 over the canonical string

```text
CodecConfig(bit_width={b},seed={s},dimension={d},residual_enabled={r})
```

where `{r}` is `str(bool)` on the Python side. `str(True)` is `"True"`,
not `"true"`, and hashing the lowercase variant silently produces a
different digest that still *looks* plausible in hex. The Rust
implementation hard-codes

```rust
r = if residual_enabled { "True" } else { "False" }
```

and the 120-triple fixture catches any drift in either direction. If
you ever "clean up" this to `{r}` / `{residual_enabled}` in a format
macro, Rust's `Display` for `bool` is lowercase — the test will catch
the regression, but the mistake is easy to make and worth a second
look in code review.

### faer debug-mode QR blows the Windows main thread stack

Running `cargo run -p tinyquant-core --example dump_rotation_fixture`
against a dim=64 matrix stack-overflows on a Windows main thread in a
debug build. Release builds survive; the faer QR path inlines more and
reuses stack frames aggressively when optimised.

The `dump_rotation_fixture` example therefore spawns its own worker with
an explicit 32 MiB stack via `std::thread::Builder`. This is the only
place in the crate that cares about main-thread stack size; unit tests
already run inside cargo's test harness (which uses larger stacks by
default on Windows) and so are unaffected.

Rule of thumb: any binary that calls into `faer::Mat::qr` at
dim ≥ 64 should either ship as a release binary or run on a worker
thread with a non-default stack.

### `cargo xtask` alias was silently broken

The pre-Phase-13 `.cargo/config.toml` had

```toml
[alias]
xtask = "run --manifest-path ${CARGO_WORKSPACE_DIR}/xtask/Cargo.toml --"
```

`${CARGO_WORKSPACE_DIR}` is not a cargo alias substitution variable —
cargo passes the literal string to the shell, which produces
`manifest path \${CARGO_WORKSPACE_DIR}/xtask/Cargo.toml does not exist`.
The alias had probably never been exercised end-to-end since the Phase 11
scaffold landed.

Fixed to the simpler form

```toml
[alias]
xtask = "run --package xtask --"
```

which works from anywhere inside the workspace and has no environment
dependency.

### `hex::encode` under `no_std` needs the `alloc` feature

`hex = { default-features = false }` gives you the `hex` namespace but
no allocating encoder. The fix is

```toml
hex = { version = "0.4", default-features = false, features = ["alloc"] }
```

in the workspace `[workspace.dependencies]`. Every dependent crate then
just writes `hex = { workspace = true }`.

### Clippy pedantic + nursery trip three non-obvious lints in rotation code

1. `clippy::float_cmp` — the `ChaChaGaussianStream` determinism unit
   test originally compared `f64` samples directly. Fixed by comparing
   `a.to_bits() != b.to_bits()`, which is strictly stronger than `!=`
   on normal NaN-free samples and is what we actually want for a
   bit-parity check.
2. `clippy::needless_range_loop` — the natural "for i in 0..dim { for j in 0..dim { ... } }"
   shape for matrix multiply and orthogonality checks is flagged. The
   rewrite uses `self.matrix.chunks_exact(dim).zip(output.iter_mut())`
   for the row-major matmul and a small `Vec<f64>` scratch buffer for
   the transpose path to preserve `f64` accumulation precision. Do
   **not** simply fold the accumulator into an `f32` per step — that
   breaks the 1e-5 round-trip budget.
3. `clippy::missing_const_for_fn` — `config_hash(&self) -> &ConfigHash`
   is const-eligible and `clippy::nursery` demands it. The `const fn`
   spelling is easy once you've seen the error; the fix is in
   `codec_config.rs`.

## faer 0.19 QR API cheat sheet

For any follow-up Rust phases that need linear algebra primitives, the
faer 0.19 API we ended up using is:

```rust
use faer::Mat;

let a = Mat::<f64>::from_fn(dim, dim, |i, j| buffer[i * dim + j]);
let qr = a.qr();
let q: Mat<f64> = qr.compute_q();
let r: Mat<f64> = qr.compute_r();

let diag_jj: f64 = r[(j, j)];          // element read via IndexOp
let q_ij: f64   = q[(i, j)];
```

Key properties:

- `Mat<f64>` is stored column-major internally; the `from_fn` closure
  receives `(row, col)` and we feed it from a row-major buffer by hand.
- `mat[(i, j)]` is an inherent index op, *not* a slice index — it does
  not trip `clippy::indexing_slicing`.
- `qr.compute_q()` and `qr.compute_r()` allocate and return fresh
  `Mat<f64>` values. At dim=768 that is ~4.7 MiB per call; we call them
  once per build and discard after the row-major copy, which fits
  comfortably within the default test-thread stack on Windows.

## Verification snapshot

At the point of landing:

- `cargo xtask fmt` clean.
- `cargo xtask lint` clean (clippy `-D warnings` with
  `clippy::all`, `clippy::pedantic`, `clippy::nursery` denied at the
  crate root, tested via the workspace-wide xtask).
- `cargo xtask test` — 71 tests passing across `tinyquant-core` and its
  siblings.
- `cargo build -p tinyquant-core --no-default-features` — green.
- `cargo build -p tinyquant-core --target thumbv7em-none-eabihf --no-default-features` — green.
- `cargo xtask fixtures refresh-all` is idempotent — `md5sum` of the
  fixture files is unchanged before and after a fresh run.
- Python spot-check:
  `hashlib.sha256(b'CodecConfig(bit_width=4,seed=42,dimension=768,residual_enabled=True)').hexdigest()`
  matches the Rust-side constant for the same triple.

## Design invariants now locked by code and fixtures

These are no longer hypothetical — they are enforced by the test suite,
so any future refactor that touches them must update the fixtures in the
same commit.

- **Canonical hash string format.** `CodecConfig(bit_width={b},seed={s},dimension={d},residual_enabled={True|False})`.
- **RNG pipeline.** `ChaCha20Rng::seed_from_u64(seed)` → 53-bit uniform
  in `[0, 1)` via `((n >> 11) as f64) * 2^-53` → polar Box-Muller with
  a cached spare.
- **QR post-processing.** `faer::Mat::<f64>::from_fn` of the row-major
  buffer → `a.qr()` → `qr.compute_q()` + `qr.compute_r()` → sign
  correction `Q[:, j] *= sign(R[j, j])` with the convention
  `sign(0) = 1`.
- **Storage.** Row-major `Arc<[f64]>`, length `dim * dim`. Column-major
  indexing would silently break the fixture bit-parity test.
- **Apply semantics.** `apply_into` promotes every `f32` input to `f64`,
  computes the whole accumulator in `f64`, and casts back to `f32` once
  per output element. `apply_inverse_into` uses a `Vec<f64>` scratch
  buffer to accumulate the transposed matmul in `f64` before the final
  `f32` cast.
- **Cache shape.** `spin::Mutex<Vec<RotationMatrix>>` with default
  capacity `8`. Linear scan on hit, LRU eviction on insert, capacity `0`
  legally bypasses the cache entirely.

## Carryover into Phase 14 and beyond

- **Phase 14 (codebook + quantize/dequantize)** consumes `RotationMatrix`
  via `RotationCache::get_or_build(config.seed(), config.dimension())`.
  The quantize kernels should feed their f32 inputs through
  `apply_into` with a scratch output buffer owned by the codec service,
  not allocate inside the hot path.
- **Phase 15 (codec service + residual)** is the right place to ship
  the legacy-vs-canonical cosine parity harness: a round-trip test
  that encodes Python-compatible bytes from Rust and asserts
  cosine-similarity agreement within the `1e-5` budget from
  [[design/rust/numerical-semantics|Numerical Semantics]].
- **Phase 15+** can promote the Rust canonical path into Python via a
  `tinyquant-py::_install_canonical_rotation(seed, dim)` entry point
  as sketched in §R1 Step 3 of
  [[design/rust/risks-and-mitigations|Risks and Mitigations]].
- **Fixture coverage** is currently `(42, 64)` and `(42, 768)`. If a
  future phase demonstrates flakiness for other `(seed, dim)` pairs,
  extend `ROTATION_GOLD_SET` in `rust/xtask/src/main.rs` and rerun
  `cargo xtask fixtures refresh-rotation` from the `rust/` directory.

## See also

- [[plans/rust/phase-13-rotation-numerics|Phase 13 Plan]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/risks-and-mitigations|Risks and Mitigations]]
- [[design/rust/memory-layout|Memory Layout]]
- [[design/rust/type-mapping|Type Mapping]]
- [[plans/rust/phase-12-shared-types-and-errors|Phase 12 Plan]]
- [[plans/rust/phase-14-codebook-quantize|Phase 14 Plan]]
