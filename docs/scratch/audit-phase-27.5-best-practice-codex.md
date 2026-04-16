---
title: Phase 27.5 Branch Audit — Best Practice and Code Smells
tags:
  - audit
  - review
  - rust
  - gpu
  - code-smells
  - best-practice
  - phase-27.5
date-created: 2026-04-16
status: active
category: scratch
---

# Phase 27.5 Branch Audit — Best Practice and Code Smells

Scope:

- branch `feature/phase-27.5-resident-corpus-search` against `develop`
- implementation inspected from detached worktree
  `TinyQuant/.worktrees/phase-27.5-audit`
- focus area: `rust/crates/tinyquant-gpu-wgpu/**` against
  [[plans/rust/phase-27.5-resident-corpus-search]]

## Findings

### 1. High — shape-only corpus idempotency hides stale-data bugs

Files:

- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:90-109`
- `docs/plans/rust/phase-27.5-resident-corpus-search.md:228-231`

`prepare_corpus_for_device` treats "same shape" as "same corpus" and returns
early when `n_rows` and `cols` match. That means a caller cannot replace a
resident corpus with new embeddings if the dimensions stay the same. GPU search
will silently serve stale scores from the old device buffer.

This is both a correctness defect and a state-management smell:

- correctness: results can be wrong with no error or invalidation signal
- maintainability: the API contract is weaker than the plan, which only blesses
  idempotency for the same slice pointer and length

Smallest safe fix:

- make idempotency explicit on pointer plus length, or
- always re-upload on every call unless a stronger identity token is supplied

### 2. Medium — the search hot path still rebuilds the pipeline on every query

Files:

- `rust/crates/tinyquant-gpu-wgpu/src/pipelines/search.rs:14-19`
- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:179-180`
- `docs/plans/rust/phase-27.5-resident-corpus-search.md:53`
- `docs/plans/rust/phase-27.5-resident-corpus-search.md:66`

The plan called for a cached `cosine_topk` pipeline. The branch adds a pipeline
builder, but not a cache. `cosine_topk` rebuilds the compute pipeline on every
search.

Why this matters:

- repeated resident-corpus queries are the whole point of the phase
- the benchmark measures `backend.cosine_topk(&query, 10)` directly, so it
  includes pipeline creation overhead
- the same crate already documents pipeline caching as a future need in the
  compress and decompress paths, so this branch adds another uncached hot path

This is not just a performance nit. It is plan drift on the branch's primary
throughput story.

### 3. Medium — host-side ranking bypasses the shared `SearchResult` ordering contract, and the tests do not catch it

Files:

- `rust/crates/tinyquant-core/src/backend/protocol.rs:30-45`
- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:270-294`
- `rust/crates/tinyquant-gpu-wgpu/tests/parity_search.rs:88-100`

`SearchResult` defines ordering as:

- descending by score
- then `vector_id` ascending as the tiebreaker

The GPU path does not use that contract. It sorts raw `(score, row)` tuples by
score only, then stringifies `row` afterward. Equal-score cases therefore keep
numeric row order, not `vector_id` lexical order. `"10"` versus `"2"` is the
obvious example.

The new parity test only compares `HashSet`s of ids, so it cannot detect ranked
order drift at all.

Maintenance cost:

- callers cannot rely on a uniform ordering model across CPU and GPU paths
- tie-handling bugs are likely to reappear because the tests encode set parity,
  not result parity

### 4. Medium — `cosine_topk` diverges from the repo's established search semantics for `top_k == 0`

Files:

- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:170-174`
- `rust/crates/tinyquant-core/src/backend/protocol.rs:95-108`
- `rust/crates/tinyquant-bruteforce/src/backend.rs:75-78`

The shared backend protocol says `top_k == 0` is an error, and the reference
brute-force backend returns `Err(BackendError::InvalidTopK)` accordingly. The
new GPU convenience API instead returns `Ok(Vec::new())`.

This is API drift, not just a taste issue:

- adjacent search surfaces in the repo now disagree on invalid input behavior
- tests do not pin the GPU behavior either way
- downstream callers have to special-case GPU search semantics that should be
  uniform across backends

`cosine_topk` is not itself a `SearchBackend` implementation, so this is not a
formal trait violation. But if it is intentionally meant to differ from the
repo's normal search semantics, that distinction needs to be documented
explicitly and tested. Right now it reads like "search, but a little
different."

### 5. Low — `backend.rs` is becoming a shotgun-surgery hotspot

Files:

- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:149-295`
- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:351-357`
- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:513-565`
- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:643-648`
- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:789-805`

The new `cosine_topk` method repeats the same orchestration shape already
present in `compress_batch` and `decompress_batch_into`:

- pipeline acquisition
- temporary buffer creation
- command encoding and submission
- blocking `map_async` readback
- host-side materialization

That is a real code smell now, not a hypothetical one. Any future work on:

- pipeline caching
- async execution
- tracing/metrics
- readback error handling
- buffer reuse

will require coordinated edits across several long methods in the same file.

Smallest cleanup:

- extract shared helpers for staging-buffer creation and readback mapping
- move pipeline caches onto `WgpuBackend` or `WgpuContext`
- keep the public methods focused on argument validation plus pass wiring

### 6. Low — the throughput bench does not exercise the documented corpus contract

Files:

- `rust/crates/tinyquant-gpu-wgpu/src/backend.rs:137-139`
- `rust/crates/tinyquant-gpu-wgpu/benches/throughput_search.rs:31-42`

The search docs say corpus and query vectors must be pre-normalized so the
shader's dot product equals cosine similarity. The benchmark normalizes the
query, but not the corpus.

That probably does not move the kernel timing much, because normalization would
happen before the timed loop anyway. But it is sloppy evidence: the benchmark is
not actually running the public API under its own documented semantic contract.

## Verification

Executed in the detached worktree:

- `cargo +1.87.0 test -p tinyquant-gpu-wgpu`
- `cargo +1.87.0 check -p tinyquant-gpu-wgpu --benches`

Both passed.

Also attempted, matching the plan's acceptance criterion:

- `cargo +1.87.0 clippy -p tinyquant-gpu-wgpu --all-targets -- -D warnings`
- `cargo +1.87.0 clippy -p tinyquant-gpu-wgpu -- -D warnings`

Both clippy invocations currently fail before reaching a clean verdict for
`tinyquant-gpu-wgpu`, because `tinyquant-core` has pre-existing deny-by-default
lint failures outside this branch diff, including:

- `manual_div_ceil` in
  `rust/crates/tinyquant-core/src/codec/compressed_vector.rs:129`
- `doc_overindented_list_items` in
  `rust/crates/tinyquant-core/src/codec/kernels/scalar.rs`
- `struct_field_names` in
  `rust/crates/tinyquant-core/src/corpus/aggregate.rs:85`

So the branch cannot presently claim the plan's clippy acceptance criterion from
a clean baseline, but that failure is not introduced by the Phase 27.5 files
themselves.

## Overall Assessment

The branch is close functionally, but not clean on best-practice grounds yet.
The biggest gaps are hidden state invalidation, search-contract drift, and the
continued growth of `backend.rs` as a multi-responsibility hot spot.
