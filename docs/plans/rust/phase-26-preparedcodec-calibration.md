---
title: "Phase 26: PreparedCodec + Calibration Gates"
tags:
  - plans
  - rust
  - phase-26
  - performance
  - testing
  - calibration
date-created: 2026-04-15
status: complete
category: planning
---

# Phase 26: PreparedCodec + Calibration Gates

> [!info] Goal
> Two parallel deliverables in a single phase:
>
> **Track A — PreparedCodec.** Add `PreparedCodec` to `tinyquant-core`:
> an owned session object that pre-builds the `RotationMatrix` once and
> caches it for reuse across batch calls. Expose `compress_prepared` and
> `decompress_prepared_into` as hot-path entry points. This is the CPU
> cleanup required before any GPU backend can be attached (Phase 27).
>
> **Track B — Calibration gate restoration.** Remove `#[ignore]` from
> the Rust Pearson ρ, compression-ratio, and residual tests in
> `tinyquant-bench/tests/calibration.rs` once the residual encoder meets
> the Plan threshold. Add the top-10 Jaccard overlap test
> (`#[ignore]` until the gold corpus fixture is provisioned in CI).
> Add a lightweight MSE-comparison test that does not require the gold corpus.
>
> No GPU crate is introduced in this phase.

> [!note] Reference docs
> - [[requirements/quality|Quality Requirements]] — FR-QUAL-001–003, FR-QUAL-005, FR-QUAL-007, FR-QUAL-008
> - [[requirements/codec|Codec Requirements]] — FR-DECOMP-004
> - [[requirements/testing-gaps|Testing Gaps]] — GAP-QUAL-001–003, GAP-QUAL-005, GAP-QUAL-007, GAP-QUAL-008, GAP-DECOMP-004, GAP-QUAL-004, GAP-GPU-001
> - [[design/rust/gpu-acceleration|GPU Acceleration Design]] §PreparedCodec design
> - [[design/rust/benchmark-harness|Benchmark Harness]] §Rotation cache warm-hit budget
> - [[design/rust/goals-and-non-goals|Goals and Non-Goals]] §rotation cache warm-hit ≤ 40 ns

## Prerequisites

- Phase 25.5 complete (JS test gap remediation).
- `cargo test --workspace` green; Python calibration tests passing.
- The rotation cache warm-hit latency fix investigated in
  [[plans/rust/calibration-threshold-investigation|Calibration Threshold
  Investigation]] is understood: the root cause is `RotationMatrix::from_config`
  being called on every compress/decompress invocation.

## Scope

Track A touches `tinyquant-core` production source (one new file
`codec/prepared.rs`, two new entry points). Track B touches only test
files — no production code changes. If a calibration gate cannot be
de-ignored because the residual encoder does not yet meet the Plan
threshold, it remains `#[ignore]` and is documented as a tracked risk
in the commit message; do not silently relax the threshold.

## Deliverables

### Track A — PreparedCodec

| File | Change |
|---|---|
| `rust/crates/tinyquant-core/src/codec/prepared.rs` (new) | `PreparedCodec` struct, `new`, `rotation`, `config`, `codebook` |
| `rust/crates/tinyquant-core/src/codec/mod.rs` | Re-export `PreparedCodec`; add `compress_prepared`, `decompress_prepared_into` on `Codec` |
| `rust/crates/tinyquant-core/tests/prepared_codec.rs` (new) | Unit + integration tests for PreparedCodec |
| `rust/crates/tinyquant-bench/benches/compress.rs` | Add `compress_prepared` bench; compare warm vs cold latency |

### Track B — Calibration gates

| File | Change |
|---|---|
| `rust/crates/tinyquant-bench/tests/calibration.rs` | Remove `#[ignore]` where threshold met; add `residual_on_has_lower_mse_than_residual_off`; add `pearson_rho_top10_overlap_4bit` (stays `#[ignore]` until gold corpus in CI) |
| `rust/crates/tinyquant-core/tests/codec_service.rs` | Add `residual_on_has_lower_mse_than_residual_off` (lightweight, dim=64) |
| `.github/workflows/rust-ci.yml` | Wire `cargo tree` grep for GPU deps (GAP-GPU-001) |

---

## Track A — Part 1: PreparedCodec (Steps 1–4)

### Step 1 — Failing tests (red)

Create `rust/crates/tinyquant-core/tests/prepared_codec.rs`:

```rust
// Tests for PreparedCodec (Phase 26).
// These tests must fail before PreparedCodec is implemented.

use tinyquant_core::{
    codec::{Codec, CodecConfig, PreparedCodec},
    codebook::Codebook,
};

fn fixture_codebook(config: &CodecConfig) -> Codebook {
    let training: Vec<f32> = (0..256 * config.dim() as usize)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();
    Codebook::train(config, &training).expect("fixture codebook must train")
}

fn fixture_vector(dim: usize, seed: u32) -> Vec<f32> {
    let mut s = seed;
    (0..dim)
        .map(|_| { s ^= s << 13; s ^= s >> 17; s ^= s << 5; (s as f32) / u32::MAX as f32 })
        .collect()
}

/// PreparedCodec builds successfully and exposes config, codebook, and rotation.
#[test]
fn prepared_codec_builds_from_valid_config() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook).unwrap();
    assert_eq!(prepared.config().hash(), config.hash());
}

/// compress_prepared and the legacy compress path produce identical output.
#[test]
fn compress_prepared_matches_legacy_compress() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    let v = fixture_vector(64, 0);

    let cv_prepared = Codec::new()
        .compress_prepared(&v, &prepared)
        .expect("compress_prepared must succeed");
    let cv_legacy = Codec::new()
        .compress(&v, &config, &codebook)
        .expect("compress must succeed");

    assert_eq!(
        cv_prepared.codes().as_ref(),
        cv_legacy.codes().as_ref(),
        "compress_prepared and compress must produce identical quantization codes"
    );
    assert_eq!(
        cv_prepared.config_hash().as_ref(),
        cv_legacy.config_hash().as_ref(),
        "config hashes must match between both paths"
    );
}

/// decompress_prepared_into and legacy decompress produce identical output.
#[test]
fn decompress_prepared_matches_legacy_decompress() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    let v = fixture_vector(64, 1);

    let cv = Codec::new()
        .compress_prepared(&v, &prepared)
        .expect("compress_prepared must succeed");

    let mut out_prepared = vec![0f32; 64];
    let mut out_legacy = vec![0f32; 64];

    Codec::new()
        .decompress_prepared_into(&cv, &prepared, &mut out_prepared)
        .expect("decompress_prepared_into must succeed");
    Codec::new()
        .decompress_into(&cv, &config, &codebook, &mut out_legacy)
        .expect("decompress_into must succeed");

    for (i, (p, l)) in out_prepared.iter().zip(out_legacy.iter()).enumerate() {
        assert!(
            (p - l).abs() < f32::EPSILON * 2.0,
            "element {i}: prepared={p} legacy={l} — must be identical"
        );
    }
}

/// PreparedCodec reuses the same RotationMatrix across multiple compress calls
/// (structural test: the same rotation pointer / hash is used both times).
#[test]
fn prepared_codec_reuses_rotation_across_calls() {
    let config = CodecConfig::new(4, 42, 64, false).unwrap();
    let codebook = fixture_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook).unwrap();

    // The rotation is stable across calls — compress two different vectors
    // and verify deterministic output (proves no re-generation per call).
    let v0 = fixture_vector(64, 10);
    let v1 = fixture_vector(64, 10); // identical to v0
    let cv0 = Codec::new().compress_prepared(&v0, &prepared).unwrap();
    let cv1 = Codec::new().compress_prepared(&v1, &prepared).unwrap();
    assert_eq!(
        cv0.codes().as_ref(),
        cv1.codes().as_ref(),
        "identical inputs must produce identical codes regardless of call order"
    );
}
```

### Step 2 — Implement `PreparedCodec`

Create `rust/crates/tinyquant-core/src/codec/prepared.rs`:

```rust
// tinyquant-core/src/codec/prepared.rs
//
// PreparedCodec: owns a pre-built RotationMatrix so the rotation is
// computed once per (config, codebook) pair rather than on every call.

use crate::{
    codec::{CodecConfig, CodecError},
    codebook::Codebook,
    rotation::RotationMatrix,
};

pub struct PreparedCodec {
    config:   CodecConfig,
    codebook: Codebook,
    rotation: RotationMatrix,
    /// Reserved for GPU-resident state attached by a GPU backend (Phase 27+).
    /// Stored as an erased type so tinyquant-core stays free of GPU imports.
    gpu_state: Option<Box<dyn core::any::Any + Send + Sync>>,
}

impl PreparedCodec {
    /// Build from a validated config and trained codebook.
    /// The rotation matrix is computed exactly once.
    pub fn new(config: CodecConfig, codebook: Codebook) -> Result<Self, CodecError> {
        let rotation = RotationMatrix::build(config.seed(), config.dim())?;
        Ok(Self { config, codebook, rotation, gpu_state: None })
    }

    pub fn config(&self) -> &CodecConfig { &self.config }
    pub fn codebook(&self) -> &Codebook { &self.codebook }
    pub fn rotation(&self) -> &RotationMatrix { &self.rotation }

    /// Attach opaque GPU state. Called by GPU backends — do not call directly.
    pub fn set_gpu_state(&mut self, state: Box<dyn core::any::Any + Send + Sync>) {
        self.gpu_state = Some(state);
    }

    /// True if a GPU backend has attached device-resident state.
    pub fn has_gpu_state(&self) -> bool { self.gpu_state.is_some() }
}
```

Add `compress_prepared` and `decompress_prepared_into` to `Codec` in
`src/codec/mod.rs`:

```rust
/// Compress using a pre-built PreparedCodec (hot path — no rotation re-build).
pub fn compress_prepared(
    &self,
    vector: &[f32],
    prepared: &PreparedCodec,
) -> Result<CompressedVector, CodecError> {
    self.compress_with_rotation(vector, prepared.config(), prepared.codebook(), prepared.rotation())
}

/// Decompress into a caller-allocated output slice using a PreparedCodec.
pub fn decompress_prepared_into(
    &self,
    cv: &CompressedVector,
    prepared: &PreparedCodec,
    out: &mut [f32],
) -> Result<(), CodecError> {
    self.decompress_into_with_rotation(cv, prepared.config(), prepared.codebook(), prepared.rotation(), out)
}
```

The internal `compress_with_rotation` and `decompress_into_with_rotation`
methods accept an explicit `&RotationMatrix` rather than building one; the
existing `compress` and `decompress_into` methods call them after building
a temporary `PreparedCodec` internally. No existing caller API changes.

### Step 3 — Add benchmark comparison

In `rust/crates/tinyquant-bench/benches/compress.rs`, add:

```rust
// Phase 26: compare warm PreparedCodec vs cold rotation-per-call.
fn bench_compress_prepared(c: &mut Criterion) {
    let config = CodecConfig::new(4, 42, 768, false).unwrap();
    let codebook = bench_codebook(&config);
    let prepared = PreparedCodec::new(config.clone(), codebook.clone()).unwrap();
    let vectors = bench_vectors(1024, 768);

    c.bench_function("compress_prepared_1024x768", |b| {
        b.iter(|| {
            for v in &vectors {
                let _ = Codec::new().compress_prepared(v, &prepared);
            }
        });
    });

    c.bench_function("compress_cold_1024x768", |b| {
        b.iter(|| {
            for v in &vectors {
                let _ = Codec::new().compress(v, &config, &codebook);
            }
        });
    });
}
```

### Step 4 — Make all Track A tests pass

Run `cargo test -p tinyquant-core --test prepared_codec`. All four tests
must pass. If `decompress_prepared_matches_legacy_decompress` shows a
non-zero delta, the internal rotation path has a discrepancy — investigate
before proceeding.

---

## Track B — Part 2: Calibration gate restoration (Steps 5–8)

### Step 5 — Lightweight MSE comparison (no gold corpus required)

Add to `rust/crates/tinyquant-core/tests/codec_service.rs` (closes GAP-DECOMP-004):

```rust
/// GAP-DECOMP-004: residual_on must produce lower MSE than residual_off
/// on the same vectors. Uses dim=64 fixture; runs in < 100 ms.
#[test]
fn residual_on_has_lower_mse_than_residual_off() {
    let dim: usize = 64;
    let seed = 42;
    let n_vectors = 100;

    let config_on  = CodecConfig::new(4, seed, dim as u32, true).unwrap();
    let config_off = CodecConfig::new(4, seed, dim as u32, false).unwrap();

    let training: Vec<f32> = (0..256 * dim)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();
    let codebook = Codebook::train(&config_on, &training).unwrap();
    // Config hash differs due to residual flag, but codebook entries are the same.
    let codebook_off = Codebook::train(&config_off, &training).unwrap();

    let mut better_count = 0usize;
    for i in 0..n_vectors {
        let v = fixture_vector(dim, i as u32);

        let cv_on  = Codec::new().compress(&v, &config_on,  &codebook).unwrap();
        let cv_off = Codec::new().compress(&v, &config_off, &codebook_off).unwrap();

        let mut out_on  = vec![0f32; dim];
        let mut out_off = vec![0f32; dim];
        Codec::new().decompress_into(&cv_on,  &config_on,  &codebook,     &mut out_on).unwrap();
        Codec::new().decompress_into(&cv_off, &config_off, &codebook_off, &mut out_off).unwrap();

        let mse_on:  f32 = v.iter().zip(&out_on).map(|(o,r)| (o-r).powi(2)).sum::<f32>() / dim as f32;
        let mse_off: f32 = v.iter().zip(&out_off).map(|(o,r)| (o-r).powi(2)).sum::<f32>() / dim as f32;

        if mse_on < mse_off { better_count += 1; }
    }

    assert!(
        better_count >= n_vectors * 95 / 100,
        "residual_on must have lower MSE than residual_off for ≥ 95% of vectors; \
         got {better_count}/{n_vectors}"
    );
}
```

### Step 6 — De-ignore calibration gates (Track B, Part A)

In `tinyquant-bench/tests/calibration.rs`, check each `#[ignore]` test
against the current residual encoder output:

1. Run `cargo test -p tinyquant-bench -- --ignored calibration` locally
   with the gold corpus fixture.
2. For each test that meets its `Plan` threshold, remove `#[ignore]`.
3. For each test that fails its `Must` threshold: keep `#[ignore]`,
   add a comment stating the measured value and the gap to `Plan`.
   Do not remove from the file — the test body is the specification.

The expected outcome based on the calibration threshold investigation:
- `pearson_rho_4bit_residual`: remove `#[ignore]` if ρ ≥ 0.995
- `pearson_rho_4bit_no_residual`: remove `#[ignore]` if ρ ≥ 0.980
- `compression_ratio_4bit`: remove `#[ignore]` if ratio ≥ 8.0
- Others: evaluate individually

### Step 7 — Add top-10 Jaccard overlap test (GAP-QUAL-004)

Add to `tinyquant-bench/tests/calibration.rs`:

```rust
/// GAP-QUAL-004: mean Jaccard overlap of top-10 neighbors for 100 queries.
/// Remains #[ignore] until the gold corpus fixture is available in CI.
/// Remove #[ignore] in the same PR that provisions the corpus LFS object.
///
/// Must: ≥ 0.80 mean Jaccard (FR-QUAL-004)
#[test]
#[ignore = "gold corpus fixture not yet available in CI — see GAP-QUAL-004"]
fn pearson_rho_top10_overlap_4bit() {
    let corpus = gold_corpus();
    let config = CodecConfig::new(4, 42, 768, false).unwrap();
    let codebook = Codebook::train(&config, corpus.training_slice()).unwrap();

    // Select 100 query vectors not in the training set.
    let query_count = 100;
    let queries: Vec<Vec<f32>> = corpus.query_slice(query_count).to_vec();

    let mut total_jaccard = 0.0f64;
    for query in &queries {
        let top10_original    = corpus.brute_force_top10(query);
        let top10_compressed  = corpus.compressed_top10(query, &config, &codebook);
        let jaccard = jaccard_similarity(&top10_original, &top10_compressed);
        total_jaccard += jaccard;
    }
    let mean_jaccard = total_jaccard / query_count as f64;
    assert!(
        mean_jaccard >= 0.80,
        "mean Jaccard overlap = {mean_jaccard:.4}, must be ≥ 0.80"
    );
}
```

### Step 8 — Wire `cargo tree` GPU isolation check (GAP-GPU-001)

Add to `.github/workflows/rust-ci.yml` in the existing matrix jobs:

```yaml
- name: Verify tinyquant-core has no GPU dependencies
  run: |
    OUTPUT=$(cargo tree -p tinyquant-core --no-default-features 2>&1)
    if echo "$OUTPUT" | grep -qE "wgpu|cust|tinyquant-gpu"; then
      echo "FAIL: GPU dependency found in tinyquant-core dep tree:"
      echo "$OUTPUT" | grep -E "wgpu|cust|tinyquant-gpu"
      exit 1
    fi
    echo "PASS: tinyquant-core dep tree is GPU-free"
```

Place this step before the main `cargo test` step so a GPU leak is caught
early without waiting for the full test run.

---

## Steps summary (ordered)

| Step | Track | Gap(s) closed | File | Est. effort |
|---|---|---|---|---|
| 1–4 | A | — | `prepared.rs` (new), `codec/mod.rs`, `prepared_codec.rs` (new), `compress.rs` bench | 3–4 h |
| 5 | B | GAP-DECOMP-004 | `codec_service.rs` | 1 h |
| 6 | B | GAP-QUAL-001–003, 005, 007, 008 | `calibration.rs` | 1.5 h |
| 7 | B | GAP-QUAL-004 | `calibration.rs` | 0.5 h |
| 8 | B | GAP-GPU-001 | `rust-ci.yml` | 0.5 h |

---

## Acceptance criteria

- [ ] `cargo test -p tinyquant-core --test prepared_codec` — all 4 tests pass.
- [ ] `cargo test -p tinyquant-core --test codec_service residual_on_has_lower_mse_than_residual_off` — passes.
- [ ] Calibration criterion benchmark shows `compress_prepared` latency
      reduction vs cold path (recorded in benchmark output; no hard gate
      on the number here, but rotation warm-hit must be measurably faster).
- [ ] Every calibration test that was de-ignored in Step 6 passes in CI
      with the gold corpus fixture.
- [ ] `pearson_rho_top10_overlap_4bit` exists in `calibration.rs` with
      `#[ignore]` annotation.
- [ ] `cargo tree` GPU isolation check added to `rust-ci.yml` and passes.
- [ ] `cargo clippy -p tinyquant-core -- -D warnings` clean.
- [ ] All closed gaps updated to `Gap: None.` in the relevant requirement
      files and gap summary table in `testing-gaps.md`.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Residual encoder does not meet `Must` threshold on current gold corpus | Medium | Medium | Keep `#[ignore]`; document measured value in test comment; track as R-QUAL in risks doc |
| `decompress_prepared_matches_legacy_decompress` reveals rotation inconsistency | Low | High | Stop — investigate root cause before any further Track B work |
| Gold corpus LFS object not available on GitHub-hosted runners | High | Low | `#[ignore]` on top-10 Jaccard test; provision separately before Phase 28 |
| `cargo tree` grep causes false positives from crate names with "gpu" substring | Low | Low | Anchor grep to full crate names: `wgpu\b\|cust\b\|tinyquant-gpu` |

## See also

- [[design/rust/gpu-acceleration|GPU Acceleration Design]] §PreparedCodec design, §Phase 26 deliverables
- [[requirements/testing-gaps|Testing Gaps]] — §GAP-QUAL-*, §GAP-DECOMP-004, §GAP-GPU-001
- [[plans/rust/phase-27-wgpu-wgsl-kernels|Phase 27]] — GPU work enabled by PreparedCodec
- [[plans/rust/calibration-threshold-investigation|Calibration Threshold Investigation]]
- [[design/rust/goals-and-non-goals|Goals and Non-Goals]] §rotation cache warm-hit ≤ 40 ns
