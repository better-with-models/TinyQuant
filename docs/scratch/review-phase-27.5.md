---
title: "Code Review — Phase 27.5: Resident Corpus GPU Search"
date-created: 2026-04-16
branch: feature/phase-27.5-resident-corpus-search
commit: e78ec8e
reviewer: Claude (Sonnet 4.6)
tags: [review, rust, gpu, phase-27.5, wgpu]
status: active
category: scratch
---

# Code Review — Phase 27.5: Resident Corpus GPU Search

Reviewed commit `e78ec8e` against plan
[[plans/rust/phase-27.5-resident-corpus-search]].
Scope: all 12 changed files (555 insertions, 20 deletions).

---

## Plan compliance

All deliverables from the plan are present.  The four documented deviations
from the plan's pseudo-code (bench in `benches/` not `tests/`, import path for
`BruteForceBackend`, `vector_id` not `id`, 16-byte uniform padding) are correct
and well-documented in the commit message.

| Deliverable | Plan | Actual | Status |
|---|---|---|---|
| `shaders/cosine_topk.wgsl` | new | present | ✓ |
| `src/pipelines/search.rs` | new | present | ✓ |
| `src/backend.rs` — `cosine_topk` + `prepare_corpus_for_device` | modify | present | ✓ |
| `src/prepared.rs` — `GpuCorpusState` | modify | present | ✓ |
| `tests/parity_search.rs` | new | present | ✓ |
| `benches/throughput_search.rs` | `tests/` in plan | `benches/` (correct) | ✓ deviated correctly |

---

## Acceptance criteria status

| Criterion | Met? | Notes |
|---|---|---|
| `parity_search` tests pass on adapter | Likely yes | Tests are correct; need GPU runner to confirm |
| Jaccard ≥ 0.95 | Implemented | Threshold correct; L2 normalization done properly |
| Count test correct for both cases | Yes | `top_k ≤ n_corpus` and `top_k > n_corpus` covered |
| `cosine_topk.wgsl` in `all_shaders_compile_without_device` | Yes | `context_probe.rs:L57` |
| Throughput ≤ 5 ms (FR-GPU-004) | **NOT evidenced** | No benchmark result in commit; pipeline not cached (see risk below) |
| `clippy -D warnings` clean | Assumed | Not independently verified here |
| `tinyquant-core` no-GPU check | Yes | No changes to `tinyquant-core` |

---

## Findings

> [!warning] Risk: pipeline rebuilt on every `cosine_topk` call
>
> `backend.rs:L180`: `build_cosine_topk_pipeline(ctx)` is called inside
> `cosine_topk` — i.e., on every query.  WGSL recompilation takes O(ms).
> The same issue exists for `compress_batch` / `decompress_batch_into` and is
> acknowledged by the Phase 28 TODO comments at L354/644, but those paths do
> **not** carry a hard throughput requirement.  `cosine_topk` does (FR-GPU-004:
> ≤ 5 ms for 10 000 × dim=1536).  Pipeline compilation alone likely exceeds
> that budget.
>
> Fix: cache the `ComputePipeline` in `WgpuBackend` (a
> `search_pipeline: Option<wgpu::ComputePipeline>` field, lazily initialised in
> `cosine_topk`).  Add a TODO if deferring to Phase 28, but document that
> FR-GPU-004 cannot be verified until caching lands.

---

> [!warning] Risk: same-shape corpus replacement silently no-ops
>
> `backend.rs:L107-111`: idempotency check compares `n_rows` and `cols` only.
> Calling `prepare_corpus_for_device` with identical dimensions but **different
> content** returns `Ok(())` without re-uploading, leaving stale data on device.
>
> The doc comment (L92-93) describes the happy-path contract but does not warn
> that **same-shape + changed content = silent stale corpus**.  This will produce
> silently wrong search results.
>
> Fix (documentation): add an explicit warning to the doc comment:
> > "There is no mechanism to replace a same-shape corpus; to update content
> > with the same dimensions, drop and recreate the backend or use a force-upload
> > API (not yet provided)."
>
> Fix (API, optional): add a `force: bool` parameter or a
> `reset_corpus()` method so callers can explicitly invalidate.

---

> [!warning] Risk: `parity_search` not added to GPU CI step
>
> `.github/workflows/gpu-ci.yml:L36-37`: the step only runs `parity_compress`.
> `parity_search` — which is the primary Phase 27.5 acceptance test — is never
> executed in CI.  The `continue-on-error: true` already means GPU failures don't
> block merges; but `parity_search` isn't even scheduled to run, so it provides
> no signal on the self-hosted GPU runner.
>
> Fix: add a second step (or extend the existing `run:` command):
> ```yaml
> - name: Run parity tests (requires wgpu adapter)
>   run: |
>     cargo test -p tinyquant-gpu-wgpu --test parity_compress
>     cargo test -p tinyquant-gpu-wgpu --test parity_search
> ```

---

> [!warning] Risk: throughput bench uses un-normalized corpus
>
> `benches/throughput_search.rs:L31-35`: the corpus is raw `sin()` values, not
> L2-normalized.  The comment at L40-41 marks only the query as "pre-normalised
> as required by the kernel", not the corpus.
>
> The kernel contract requires unit-norm inputs; the parity test normalizes
> correctly (both corpus and query).  The bench therefore measures throughput
> under a condition that violates its own pre-normalization contract.
> This doesn't affect timing accuracy (the kernel does the same work), but:
>
> 1. The bench output is not a faithful representation of production behavior
>    (where corpus would be pre-normalized before upload).
> 2. If someone later adds score-correctness assertions to the bench, they
>    will silently get wrong scores.
>
> Fix: normalize each corpus row before `prepare_corpus_for_device`, matching
> the parity test pattern.

---

> [!tip] Nit: NaN sort stability in top-k selection
>
> `backend.rs:L277-281`: `partial_cmp` returns `None` for NaN comparisons; the
> `unwrap_or(Equal)` makes NaN entries sort arbitrarily.  For pre-normalized
> vectors this shouldn't occur, but a malformed query or corpus row would silently
> contaminate results.
>
> Fix: replace with `b.0.total_cmp(&a.0)` which places NaNs at the end of a
> descending sort and is stable.
>
> ```rust
> scored.sort_by(|a, b| b.0.total_cmp(&a.0));
> ```

---

> [!tip] Nit: unnecessary `as_str()` round-trip when creating `Arc<str>`
>
> `backend.rs:L291`:
> ```rust
> Arc::from(row.to_string().as_str())
> ```
> `Arc<str>` implements `From<String>` directly.  The `.as_str()` step just
> borrows and re-clones.
>
> Fix:
> ```rust
> Arc::from(row.to_string())
> ```

---

> [!tip] Nit: no assertion on result ordering in parity tests
>
> `tests/parity_search.rs`: both `gpu_top10_overlaps_cpu_top10` and
> `gpu_cosine_topk_returns_correct_count` test result set membership and count
> but not that results are sorted in descending score order.  A bug that returns
> the correct set but in arbitrary order would pass.
>
> Fix: add a one-liner after the `cosine_topk` call:
> ```rust
> let scores: Vec<f32> = results.iter().map(|r| r.score).collect();
> assert!(scores.windows(2).all(|w| w[0] >= w[1]), "results must be sorted descending");
> ```

---

> [!tip] Nit: query bandwidth not shared across workgroup threads
>
> `shaders/cosine_topk.wgsl:L34-43`: each thread reads all `dim` query elements
> independently.  With `@workgroup_size(256)`, the query vector is loaded 256
> times per workgroup.  For dim=1536 × 4 bytes = 6 KB, this is 256 × 6 KB =
> 1.5 MB of query reads per workgroup dispatch — unnecessary with shared memory.
>
> This is a known follow-up optimization (not a bug), but it is the primary
> reason FR-GPU-004 may not be met on bandwidth-constrained hardware.  The plan's
> risk table mentions "block-local sort" but not query caching.
>
> Consider noting this in the AGENTS.md "Extend corpus search" section so the
> next phase knows where the remaining bandwidth goes.

---

> [!question] Question: `GpuCorpusState.corpus_buf` uses `Buffer` not `Arc<Buffer>`
>
> `prepared.rs:L41`:
> ```rust
> pub corpus_buf: Buffer,
> ```
> All fields in `GpuPreparedState` use `Arc<Buffer>` (L20-26).  `GpuCorpusState`
> uses a plain `Buffer`.  Is this intentional?  Plain `Buffer` is correct here
> since the corpus buffer is not shared with any other state struct — just
> confirming it's a deliberate choice rather than an oversight.

---

## Summary

The implementation is clean, correct, and follows the repo's established
conventions well.  The key risks are:

1. **FR-GPU-004 is not verifiable** until `cosine_topk` caches its pipeline.
   The throughput requirement should not be declared met until a benchmark
   result is recorded on real hardware with a cached pipeline.

2. **`parity_search` is invisible to CI.**  Adding it to `gpu-ci.yml` is a
   one-liner and completes the Phase 27.5 acceptance gate.

3. **Same-shape corpus replacement** is silently a no-op; the doc comment
   needs a warning to prevent subtle production bugs.

The NaN sort and `Arc::from` nits are low priority.  The ordering assertion in
parity tests is worth adding before Phase 28 relies on `cosine_topk` ordering.
