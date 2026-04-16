---
title: "Phase 27.5: Resident Corpus GPU Search"
tags:
  - plans
  - rust
  - phase-27.5
  - gpu
  - wgpu
  - search
  - cosine-similarity
date-created: 2026-04-15
status: draft
category: planning
---

# Phase 27.5: Resident Corpus GPU Search

> [!info] Goal
> Extend `tinyquant-gpu-wgpu` with GPU-resident corpus search scoring:
> the `cosine_topk` kernel that scores a single query vector against an
> entire device-resident corpus and returns the top-k results.
>
> When the corpus stays resident on the GPU (uploaded once via
> `prepare_corpus_for_device`), repeated similarity queries avoid
> round-tripping the full corpus through PCIe. This is the phase that
> unlocks the GPU performance advantage for the primary production use
> case: embedding nearest-neighbor retrieval.
>
> The CPU `BruteForceBackend` remains the semantic truth source.
> Every GPU result is gated by a parity test against it.

> [!note] Reference docs
> - [[design/rust/gpu-acceleration|GPU Acceleration Design]] — §Kernel 3 (cosine_topk), §Memory strategy, §Synchronization model
> - [[requirements/gpu|GPU Requirements]] — FR-GPU-004 (throughput gate for Phase 27 target)
> - [[plans/rust/phase-27-wgpu-wgsl-kernels|Phase 27]] — compress/decompress prerequisite
> - [[design/rust/benchmark-harness|Benchmark Harness]] — GPU throughput bench format

## Prerequisites

- Phase 27 complete: `WgpuBackend` exists with `compress_batch`,
  `decompress_batch_into`, and `prepare_for_device`. All Phase 27
  acceptance criteria met.
- `WgpuContext` is functional; `GPU_BATCH_THRESHOLD` constant defined.
- Self-hosted GPU runner available (or `llvmpipe` software renderer
  for correctness tests; throughput bench on real GPU only).

## Scope

All changes are within `tinyquant-gpu-wgpu`. No other crate is modified.

New source files:
- `shaders/cosine_topk.wgsl` — GPU scoring kernel
- `src/pipelines/search.rs` — build + cache the cosine_topk pipeline
- `tests/parity_search.rs` — CPU vs GPU top-k parity
- `tests/throughput_search.rs` — criterion bench (FR-GPU-004 evidence)

Modified files:
- `src/backend.rs` — add `cosine_topk` and `prepare_corpus_for_device`
- `src/prepared.rs` — extend GPU state to hold corpus buffer handle

## Deliverables

| File | Change |
|---|---|
| `shaders/cosine_topk.wgsl` (new) | Block-local top-k cosine similarity kernel |
| `src/pipelines/search.rs` (new) | Pipeline cache for cosine_topk |
| `src/backend.rs` | `cosine_topk`, `prepare_corpus_for_device` on `WgpuBackend` |
| `src/prepared.rs` | Corpus buffer field in GPU state |
| `tests/parity_search.rs` (new) | CPU vs GPU top-10 overlap ≥ 95% |
| `tests/throughput_search.rs` (new) | Criterion bench: 10 000 vectors dim=1536 |

---

## Part A — Corpus residency and search pipeline (Steps 1–2)

### Step 1 — Failing tests (red)

Create `tests/parity_search.rs`:

```rust
// Parity: GPU cosine_topk must agree with CPU BruteForceBackend top-10.
use tinyquant_core::{
    backend::{BruteForceBackend, SearchBackend, SearchResult},
    codec::{Codec, CodecConfig, PreparedCodec},
    codebook::Codebook,
};
use tinyquant_gpu_wgpu::{ComputeBackend, WgpuBackend};

fn skip_if_no_adapter() -> Option<WgpuBackend> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(WgpuBackend::new()).ok()
}

/// GPU top-10 results must overlap with CPU top-10 by ≥ 95% (Jaccard).
///
/// Corpus: 1 000 decompressed FP32 vectors, dim=64.
/// 10 query vectors drawn from outside the corpus.
#[test]
fn gpu_top10_overlaps_cpu_top10() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };

    let dim = 64usize;
    let n_corpus = 1_000usize;
    let n_queries = 10usize;

    // Build corpus in FP32 (decompressed form).
    let corpus_fp32: Vec<f32> = (0..n_corpus * dim)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();

    // Prepare GPU corpus residency.
    backend.prepare_corpus_for_device(&corpus_fp32, n_corpus, dim).unwrap();

    // CPU reference: BruteForceBackend.
    let mut cpu_backend = BruteForceBackend::new();
    let entries: Vec<(String, Vec<f32>)> = (0..n_corpus)
        .map(|i| {
            let v = corpus_fp32[i * dim..(i + 1) * dim].to_vec();
            (format!("v{i}"), v)
        })
        .collect();
    cpu_backend.ingest(&entries).unwrap();

    for q in 0..n_queries {
        let query: Vec<f32> = (0..dim)
            .map(|j| ((q * dim + j) as f32 * 0.007).cos())
            .collect();

        let gpu_results: Vec<SearchResult> = backend.cosine_topk(&query, 10).unwrap();
        let cpu_results: Vec<SearchResult> = cpu_backend.search(&query, 10).unwrap();

        let gpu_ids: std::collections::HashSet<_> = gpu_results.iter().map(|r| &r.id).collect();
        let cpu_ids: std::collections::HashSet<_> = cpu_results.iter().map(|r| &r.id).collect();
        let intersection = gpu_ids.intersection(&cpu_ids).count();
        let union        = gpu_ids.union(&cpu_ids).count();
        let jaccard      = intersection as f64 / union as f64;

        assert!(
            jaccard >= 0.95,
            "query {q}: GPU vs CPU top-10 Jaccard = {jaccard:.3}, must be ≥ 0.95"
        );
    }
}

/// GPU cosine_topk returns exactly top_k results (or all corpus rows if
/// n_corpus < top_k).
#[test]
fn gpu_cosine_topk_returns_correct_count() {
    let Some(mut backend) = skip_if_no_adapter() else {
        eprintln!("SKIP: no wgpu adapter");
        return;
    };
    let dim = 32usize;
    let n_corpus = 20usize;
    let corpus: Vec<f32> = (0..n_corpus * dim).map(|i| i as f32 * 0.01).collect();
    backend.prepare_corpus_for_device(&corpus, n_corpus, dim).unwrap();

    let query: Vec<f32> = vec![1.0f32; dim];
    let results = backend.cosine_topk(&query, 10).unwrap();
    assert_eq!(results.len(), 10, "must return exactly 10 results from a 20-row corpus");

    // Request more than corpus size — should return all rows.
    let results_all = backend.cosine_topk(&query, 50).unwrap();
    assert_eq!(results_all.len(), n_corpus, "top_k > n_corpus must return all rows");
}
```

### Step 2 — `cosine_topk.wgsl` kernel

Write `shaders/cosine_topk.wgsl` following the two-phase (block top-k + host
merge) strategy from the GPU Acceleration Design §Kernel 3:

```wgsl
// shaders/cosine_topk.wgsl
//
// Phase 1: each workgroup computes cosine similarity for its slice of corpus
// rows and writes (score, row_index) pairs to a per-workgroup output buffer.
// Phase 2 (host side): merge per-workgroup top-k lists to find global top-k.
//
// Cosine similarity = dot(q, r) / (|q| * |r|).
// All vectors are pre-normalized to unit length on the host before upload
// (this avoids a division per kernel invocation). If vectors are not
// pre-normalized, set PRENORMALIZED = false and enable the division branch.

struct Dims {
    n_rows:    u32,   // corpus row count
    dim:       u32,   // embedding dimension
    top_k:     u32,   // results to return per workgroup
}

@group(0) @binding(0) var<uniform>             dims:   Dims;
@group(0) @binding(1) var<storage, read>       corpus: array<f32>; // n_rows × dim
@group(0) @binding(2) var<storage, read>       query:  array<f32>; // dim
@group(0) @binding(3) var<storage, read_write> scores: array<f32>; // n_rows

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let row = gid.x;
    if (row >= dims.n_rows) { return; }

    var dot: f32 = 0.0;
    let offset = row * dims.dim;
    for (var d: u32 = 0u; d < dims.dim; d++) {
        dot += corpus[offset + d] * query[d];
    }
    // Pre-normalized path: score = dot product.
    scores[row] = dot;
}
```

> [!note] Simplification: host-side top-k merge
> The first release uses a single-pass kernel that writes all cosine scores,
> then performs top-k selection on the host (CPU sort of `scores`). This
> avoids the complexity of a parallel block-local sort at the cost of
> reading back `n_rows` floats per query. For 10 000 rows × 4 bytes = 40 KB,
> the readback is dominated by PCIe overhead not the copy itself.
>
> A future optimisation can add a block-level partial sort to the kernel
> to reduce readback to `n_blocks × top_k` floats. Profile before adding
> that complexity.

Add `prepare_corpus_for_device` and `cosine_topk` to `src/backend.rs`:

```rust
/// Upload a decompressed FP32 corpus to device-resident memory.
/// Must be called before cosine_topk. Idempotent: re-uploading the same
/// corpus (same slice pointer and length) returns Ok(()) without re-upload
/// if the buffer already exists.
pub fn prepare_corpus_for_device(
    &mut self,
    corpus: &[f32],
    rows: usize,
    cols: usize,
) -> Result<(), TinyQuantGpuError>;

/// Score `query` (len = cols) against the resident corpus and return
/// the top-k results sorted by descending cosine similarity.
/// Panics if prepare_corpus_for_device has not been called.
pub fn cosine_topk(
    &self,
    query: &[f32],
    top_k: usize,
) -> Result<Vec<tinyquant_core::backend::SearchResult>, TinyQuantGpuError>;
```

---

## Part B — Throughput benchmark (Step 3)

### Step 3 — Criterion bench for FR-GPU-004

Create `tests/throughput_search.rs` (criterion bench, not `#[test]`):

```rust
// FR-GPU-004 evidence: GPU cosine_topk for 10 000 vectors dim=1536 in ≤ 5 ms.
// Run on a self-hosted GPU runner (RTX 3060 class or better).
use criterion::{criterion_group, criterion_main, Criterion};
use tinyquant_gpu_wgpu::WgpuBackend;

fn bench_cosine_topk(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let backend = match rt.block_on(WgpuBackend::new()) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("SKIP throughput bench: no wgpu adapter");
            return;
        }
    };

    let dim = 1536usize;
    let n_corpus = 10_000usize;
    let corpus: Vec<f32> = (0..n_corpus * dim)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();

    let mut backend = backend;
    rt.block_on(async { backend.prepare_corpus_for_device(&corpus, n_corpus, dim).unwrap() });

    let query: Vec<f32> = vec![1.0 / (dim as f32).sqrt(); dim]; // unit vector

    c.bench_function("cosine_topk_10k_dim1536", |b| {
        b.iter(|| {
            backend.cosine_topk(&query, 10).unwrap();
        });
    });
}

criterion_group!(benches, bench_cosine_topk);
criterion_main!(benches);
```

Add to `Cargo.toml`:

```toml
[[bench]]
name    = "throughput_search"
harness = false

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
tokio     = { version = "1", features = ["rt"] }
```

---

## Steps summary

| Step | Gap(s) closed | File | Est. effort |
|---|---|---|---|
| 1–2 | FR-GPU-004 (evidence gathered) | `parity_search.rs`, `cosine_topk.wgsl`, `backend.rs`, `prepared.rs` | 4–6 h |
| 3 | — | `throughput_search.rs` criterion bench | 1 h |

No new GAP-* tags are introduced or closed by this phase beyond those
already assigned to Phase 27. This phase completes the GPU search path
needed before Phase 28 (CUDA) can be benchmarked against a common baseline.

---

## Acceptance criteria

- [ ] `cargo test -p tinyquant-gpu-wgpu --test parity_search` — all tests
      pass on a runner with a wgpu-compatible adapter.
- [ ] `gpu_top10_overlaps_cpu_top10`: Jaccard ≥ 0.95 for all 10 queries.
- [ ] `gpu_cosine_topk_returns_correct_count`: result count correct for
      both `top_k ≤ n_corpus` and `top_k > n_corpus` cases.
- [ ] `cosine_topk.wgsl` validates without errors in `all_shaders_compile_without_device`.
- [ ] Throughput bench run on GPU runner; result recorded in commit message.
      **Must:** median ≤ 5 ms for 10 000 rows × dim=1536 (FR-GPU-004).
      If not met, document the gap and open a tracked risk before declaring
      Phase 27.5 complete.
- [ ] `cargo clippy -p tinyquant-gpu-wgpu -- -D warnings` clean.
- [ ] `tinyquant-core` no-GPU check (from Phase 26 CI) still passes.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| FR-GPU-004 throughput not met on first implementation | Medium | Medium | Profile: isolate PCIe transfer vs kernel time. If kernel is fast but transfer is the bottleneck, async transfer overlapping or persistent corpus buffers address it |
| `prepare_corpus_for_device` idempotency check is expensive | Low | Low | Compare buffer size and a hash of the corpus pointer + length, not content; document the contract |
| Host-side top-k sort dominates at n_rows > 100k | Low | Low | Document threshold; add block-local sort in a follow-up if needed |
| wgpu pre-normalization assumption wrong for some corpora | Medium | Medium | Add optional in-kernel norm; document that users must pre-normalize or pass a flag |

## See also

- [[design/rust/gpu-acceleration|GPU Acceleration Design]] §Kernel 3, §Memory strategy
- [[requirements/gpu|GPU Requirements]] — FR-GPU-004
- [[plans/rust/phase-27-wgpu-wgsl-kernels|Phase 27]] — compress/decompress prerequisite
- [[plans/rust/phase-28-cuda-backend|Phase 28]] — CUDA specialist path
- [[design/rust/benchmark-harness|Benchmark Harness]] §GPU throughput
