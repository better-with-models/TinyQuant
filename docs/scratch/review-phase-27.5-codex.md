---
title: Phase 27.5 Branch Review
tags:
  - review
  - rust
  - gpu
  - phase-27.5
  - wgpu
date-created: 2026-04-16
status: active
category: scratch
---

# Phase 27.5 Branch Review

Reviewed branch `feature/phase-27.5-resident-corpus-search` against `develop`
and against [[plans/rust/phase-27.5-resident-corpus-search]].

## Findings

> [!danger] High — same-shape corpus refreshes are silently ignored, so GPU search can serve stale data
>
> File: `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:90-109`
>
> The implementation treats "same shape" as "same corpus":
>
> - the doc comment says re-upload is skipped when the same `n_rows × cols`
>   shape is already resident
> - the code returns early when `s.n_rows == n_rows && s.cols == cols`
> - no pointer, checksum, or content comparison is performed
>
> That means a caller cannot replace a corpus with new data if the row count and
> dimension stay the same. `cosine_topk` will keep reading the old device buffer
> and return stale results with no error.
>
> This is also looser than the plan contract. The plan only blesses idempotency
> for re-uploading the same corpus slice, defined as the same slice pointer and
> length: [[plans/rust/phase-27.5-resident-corpus-search]]:228-231.
>
> Impact:
>
> - correctness bug for any workflow that refreshes embeddings in place
> - hard to detect because the API reports success
> - current tests do not cover it

> [!warning] Medium — the search pipeline is not cached, so the hot path still pays per-query pipeline build cost
>
> Files:
>
> - `rust/crates/tinyquant-gpu-wgpu/src/pipelines/search.rs:14-19`
> - `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:179-180`
> - `rust/crates/tinyquant-gpu-wgpu/benches/throughput_search.rs:44-47`
>
> The plan explicitly calls for `src/pipelines/search.rs` to "build + cache the
> cosine_topk pipeline":
>
> - [[plans/rust/phase-27.5-resident-corpus-search]]:53
> - [[plans/rust/phase-27.5-resident-corpus-search]]:66
>
> The branch does not implement that cache. `cosine_topk` rebuilds the compute
> pipeline on every call via `build_cosine_topk_pipeline(ctx)`.
>
> Why this matters:
>
> - the phase goal is repeated resident-corpus queries
> - the throughput bench times `backend.cosine_topk(&query, 10)` directly
> - that measurement therefore includes per-call pipeline creation overhead, not
>   just steady-state search
>
> This does not break correctness, but it weakens the branch against the
> phase goal and makes FR-GPU-004 evidence less trustworthy.

> [!warning] Medium — GPU result ordering violates the shared `SearchResult` ordering contract on ties and NaNs
>
> Files:
>
> - `rust/crates/tinyquant-core/src/backend/protocol.rs:30-45`
> - `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:270-281`
> - `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:287-294`
> - `rust/crates/tinyquant-gpu-wgpu/tests/parity_search.rs:88-99`
>
> `SearchResult` has an explicit ordering contract:
>
> - descending by score
> - then `vector_id` ascending as a tiebreaker
>
> The GPU path sorts only by score:
>
> - equal scores fall through as `Equal`
> - stable sort then preserves raw row order
> - only after sorting does the code materialize `vector_id` strings from row
>   numbers
>
> That differs from the core contract for many equal-score cases. Example:
>
> - row `2` gets id `"2"`
> - row `10` gets id `"10"`
> - numeric row order is `2, 10`
> - lexicographic `vector_id` order is `"10", "2"`
>
> So GPU search can disagree with CPU backends even when all scores are valid and
> equal. The new parity test only compares sets, not ranked order, so it will not
> catch this.

> [!tip] Low — the throughput bench does not follow the search API's own pre-normalization contract
>
> Files:
>
> - `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:137-139`
> - `rust/crates/tinyquant-gpu-wgpu/benches/throughput_search.rs:31-42`
>
> The public search path documents a pre-normalized corpus/query contract so that
> the shader's dot product equals cosine similarity. The benchmark normalizes the
> query, but it uploads an unnormalized sinusoid corpus.
>
> This probably does not change the measured kernel cost in a meaningful way,
> because normalization would happen before the timed loop anyway. But it does
> mean the benchmark is not measuring the public API under its documented
> semantic contract, which is sloppy for FR-GPU-004 evidence.

## Verification

Confirmed locally with Rust 1.87:

- `cargo +1.87.0 test -p tinyquant-gpu-wgpu`
- `cargo +1.87.0 check -p tinyquant-gpu-wgpu --benches`

Both passed.

The workspace default toolchain is still pinned to Rust 1.81 via
`rust/rust-toolchain.toml`, so plain `cargo test -p tinyquant-gpu-wgpu` fails
unless the command explicitly selects `+1.87.0`. I treated that as an
environment/workspace constraint, not a branch-specific review finding, because
this diff did not change the toolchain file.

## Residual Risks

- No regression test covers replacing a resident corpus with new same-shape data.
- No test checks ranked-order parity for equal scores or NaNs.
- The branch adds a benchmark harness, but not benchmark results, so FR-GPU-004
  is not demonstrated by the branch contents alone.
