---
title: Storage Codec Vs Search Backend Separation
tags:
  - concept
  - architecture
  - compression
  - search
date-created: 2026-04-08
status: active
category: concept
source-count: 2
---

# Storage Codec Vs Search Backend Separation

> [!question] Why is this important?
> TinyQuant's research repeatedly treats compression and search as separate
> responsibilities. That boundary is central to both correctness and
> performance.

## Core idea

TinyQuant should own vector compression, decompression, serialization, and
codec-level invariants. It should not own approximate nearest-neighbor search
or production retrieval infrastructure.

## Why this boundary exists

- The research baseline shows compressed-domain search is too slow on CPU
- Score fidelity remains strong when vectors are decompressed back to FP32
  before similarity search
- Different consumers may want different search backends such as brute-force,
  pgvector, or FAISS
- Keeping search out of the codec preserves a small and reusable public API

## Architectural consequence

The correct mental model is:

```text
write path: FP32 vector -> TinyQuant compression -> durable storage
read path: stored representation -> TinyQuant decompression -> backend search
```

This means TinyQuant is a **storage-layer optimization** rather than a search
subsystem.

## Why this matters for project scope

This separation keeps TinyQuant:

- reusable across multiple storage systems
- easier to validate against clean-room research baselines
- less likely to over-promise retrieval quality
- aligned with the deployment story described for better-router and TurboSwede

## See also

- [[TinyQuant]]
- [[tinyquant-library-research]]
- [[tinyquant-better-router-integration]]
- [[storage-codec-architecture]]
