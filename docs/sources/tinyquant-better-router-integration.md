---
title: TinyQuant Better Router Integration
tags:
  - source
  - tinyquant
  - better-router
  - integration
date-created: 2026-04-08
status: active
source-type: research-summary
source-count: 1
---

# TinyQuant Better Router Integration

> [!info] Source summary
> Summary of `docs/research/tinyquant-research/better-router-integration.md`.

## Core claim

The researched integration path places TinyQuant behind TurboSwede. The router
 should not call TinyQuant directly; compression and decompression belong to the
 storage pipeline of the context database.

## Recommended deployment order

1. Gold corpus collections, where score fidelity matters more than exact top-1
2. Orchestrator-owned collections, where structured filters dominate retrieval
3. Decision memory and context recall, conditionally, once operational evidence
   shows compression noise is acceptable

## Operational model

- TinyQuant participates primarily on the write path by compressing FP32
  embeddings before durable storage
- Search remains FP32 by using decompressed or materialized shadow columns
- This keeps TinyQuant off the latency-critical hot path for retrieval

## Important design consequences

- [[TinyQuant]] should expose a backend protocol rather than owning search
- [[storage-codec-vs-search-backend-separation]] is not an implementation
  detail; it is the main safety boundary for production use
- [[per-collection-compression-policy]] is a first-class operational control
  because not every collection should be compressed the same way

## Risk posture

- Storage-heavy, score-correlation-driven collections are the best first fit
- Exact-match retrieval workloads remain the most sensitive area
- Materialized decompression is the main mitigation for read-path latency and
  fidelity concerns

## See also

- [[TinyQuant]]
- [[storage-codec-vs-search-backend-separation]]
- [[per-collection-compression-policy]]
- [[storage-codec-architecture]]
