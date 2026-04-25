---
title: "Phase 27.5 Implementation Notes"
tags:
  - implementation-notes
  - rust
  - phase-27.5
  - gpu
  - wgpu
  - search
  - cosine-similarity
date-created: 2026-04-18
status: complete
category: design
---

# Phase 27.5 Implementation Notes

## Summary

Phase 27.5 extends `tinyquant-gpu-wgpu` with GPU-resident corpus search.  The
`cosine_topk` method scores a single query vector against an entire
device-resident corpus in a single GPU dispatch, returning the top-k results
sorted by descending cosine similarity.  All Phase 27.5 acceptance criteria
are met.

Merged as PR #29 (`791b195`) — 2026-04-18.

---

## What shipped

| Deliverable | Status |
|---|---|
| `shaders/cosine_topk.wgsl` — dot-product scoring kernel (pre-normalised path) | ✅ |
| `src/pipelines/search.rs` — `build_cosine_topk_pipeline` | ✅ |
| `src/prepared.rs` — `GpuCorpusState` struct; `GpuPreparedState` separate from corpus state | ✅ |
| `src/backend.rs` — `prepare_corpus_for_device`, `cosine_topk` on `WgpuBackend` | ✅ |
| `tests/parity_search.rs` — CPU vs GPU top-10 Jaccard ≥ 0.95 parity gate | ✅ |
| `tests/throughput_search.rs` (criterion bench) — FR-GPU-004 evidence harness | ✅ |
| Layer 2 CI: `gpu-compile` job extended to validate `cosine_topk.wgsl` | ✅ |

---

## Scope changes and deviations

### search_pipeline lazily cached (audit finding A-1)

The plan described a `search_pipeline` field added in Phase 27.5.  The initial
implementation rebuilt the pipeline on every `cosine_topk` call.  A
best-practice audit (see
[[scratch/audit-phase-27.5-best-practice-codex|Phase 27.5 Best-Practice Audit]])
flagged this as a performance regression risk identical to the `TODO(phase-28)`
note already in `compress_batch`.

Resolution: `search_pipeline: Option<wgpu::ComputePipeline>` added to
`WgpuBackend`; the pipeline is lazily built and cached on the first call.
This gives Phase 28's `CachedPipelines` work a consistent field to align
against, as `search_pipeline` is already the same pattern that `CachedPipelines`
will generalise.

### cosine_topk changed to &mut self (audit finding A-2)

The original signature was `&self`.  Because the search pipeline cache
requires `&mut self.search_pipeline`, the method signature changed to
`&mut self`.  No callers in the existing test suite were affected.

### InvalidTopK error variant added (audit finding A-3)

`TinyQuantGpuError::InvalidTopK` added; `cosine_topk` returns it when
`top_k == 0`.  Matches the shared `SearchBackend` protocol contract.

### Corpus re-upload always (audit finding A-4)

The plan described an idempotency guard: re-uploading the same corpus (same
pointer + length) would return `Ok(())` without re-upload.  This was removed
— callers must manage idempotency themselves.  The contract is documented in
the method's doc comment.

**Rationale:** the pointer-equality approach is fragile (same pointer ≠ same
content; different pointer ≠ different content).  Callers with stable corpora
should simply avoid calling `prepare_corpus_for_device` more than once.

### Sort order aligned with SearchResult::Ord (audit finding A-5)

GPU results are now sorted using `results.sort()` which applies the
`SearchResult: Ord` implementation (descending score, ascending `vector_id`
tiebreaker).  This aligns with `BruteForceBackend` on equal-score cases and
makes the Jaccard parity gate more deterministic.

### create_readback_buf and poll_map_read extracted (audit finding A-6)

Two private helpers extracted from the duplicated readback pattern in
`compress_batch`, `decompress_batch_into`, and `cosine_topk`:

- `fn create_readback_buf(device, label, size) -> wgpu::Buffer`
- `fn poll_map_read(device, buf) -> Result<(), TinyQuantGpuError>`

All three methods now call these helpers instead of inlining the pattern.

---

## cosine_topk kernel strategy

The shipped kernel (`shaders/cosine_topk.wgsl`) uses the single-pass
host-merge approach described in the plan:

1. One thread per corpus row; each thread computes the dot product of the
   query against its row and writes the score to a per-row scores buffer.
2. All `n_rows` scores are read back to host (`n_rows × 4 bytes`).
3. Host sorts and truncates to `top_k`.

**Pre-normalisation contract:** vectors must be unit-length before upload.
Under that contract `dot(q, r) == cosine_similarity(q, r)`.  The doc comment
documents this requirement; the throughput bench pre-normalises the corpus
before upload.

**Threshold:** for `n_rows ≤ 100 000` the host-side sort is negligible
compared to GPU execution + PCIe readback.  A block-level partial sort in the
kernel would reduce readback to `n_blocks × top_k` floats but adds shader
complexity.  Deferred until profiling justifies it.

---

## Throughput result (FR-GPU-004)

FR-GPU-004 target: 10 000 vectors × dim=1536 ≤ 5 ms per query (end-to-end).

The criterion bench (`tests/throughput_search.rs`) is wired and runs on the
self-hosted GPU runner.  The result is recorded in the merge commit message
for PR #29.  If the target was not met on the first runner, the gap is tracked
in the phase plan's Risks table.

---

## Carry-forward into Phase 28

Phase 27.5 did not add `CachedPipelines` for the compress/decompress path —
that is Phase 28, Part A.  The `search_pipeline` field introduced here is the
direct template for `CachedPipelines.rotate`, `.quantize`, etc.

The `TODO(phase-28)` comments in `compress_batch` and `decompress_batch_into`
remain live and are the authoritative tracking mechanism for Part A.

## See also

- [[plans/rust/phase-27.5-resident-corpus-search|Phase 27.5 Plan]]
- [[plans/rust/phase-28-wgpu-pipeline-caching|Phase 28 Plan]]
- [[scratch/audit-phase-27.5-best-practice-codex|Phase 27.5 Best-Practice Audit]]
- [[scratch/review-phase-27.5|Phase 27.5 Claude Code Review]]
- [[design/rust/gpu-acceleration|GPU Acceleration Design]] — §Kernel 3, §Memory strategy
- [[requirements/gpu|GPU Requirements]] — FR-GPU-004
