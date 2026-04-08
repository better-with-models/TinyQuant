---
title: Per Collection Compression Policy
tags:
  - concept
  - operations
  - compression
date-created: 2026-04-08
status: active
category: concept
source-count: 1
---

# Per Collection Compression Policy

> [!info] Operational concept
> Not every embedding collection benefits equally from TinyQuant compression.

## Core idea

Compression should be selectable per collection rather than being forced as a
single global mode across every workload.

## Why it matters

The research distinguishes among workloads with different tolerance for
quantization noise:

- gold-corpus and calibration-style collections mainly care about score
  correlation and are strong candidates for compression
- orchestrator-owned collections often rely on structured filters, so vector
  fidelity is less critical
- decision-memory and hot-path recall workloads are more sensitive and may need
  passthrough or FP16 modes first

## Policy shape

The research suggests a policy family like:

- `compress`
- `passthrough`
- `fp16`

This keeps operational rollout reversible and lets downstream systems tune
storage pressure against retrieval sensitivity without changing TinyQuant's core
API.

## Design implication

Even if TinyQuant itself is just a codec library, its integration guidance
should assume downstream consumers may need differentiated compression policies
per collection or table.

## See also

- [[TinyQuant]]
- [[tinyquant-better-router-integration]]
- [[storage-codec-vs-search-backend-separation]]
