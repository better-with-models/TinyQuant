---
title: Wiki Index
tags:
  - meta
  - index
date-created: 2026-04-08
status: active
---

# Index

> [!info] Content catalog
> This page is the entry point for the TinyQuant wiki. Update it whenever new
> durable documentation is added.

## Sources

| Page | Summary | Date |
|------|---------|------|
| [[tinyquant-library-research]] | Source summary for the TinyQuant library research seed: codec scope, algorithm shape, quality baseline, and API direction | 2026-04-08 |
| [[tinyquant-better-router-integration]] | Source summary for the better-router and TurboSwede integration path, deployment order, and operational constraints | 2026-04-08 |
| [[vector-quantization-paper-synthesis]] | Cross-paper synthesis of QJL, PolarQuant, and TurboQuant, including the main algorithmic themes relevant to TinyQuant | 2026-04-08 |
| [[turbo-quant-deep-research-report]] | Deep research summary of TurboQuant's claims, deployment posture, and relationship to neighboring quantization methods | 2026-04-08 |
| [[turbo-quant-code-availability]] | Summary of official and community code availability across TurboQuant and related methods, with provenance implications for TinyQuant | 2026-04-08 |
| [[turboquant-integration]] | System-integration summary for bringing TurboQuant-style compression into the better-router stack | 2026-04-08 |

## Entities

| Page | Summary | Date |
|------|---------|------|
| [[TinyQuant]] | Project entity page covering TinyQuant as a CPU-only storage codec for embedding compression with pluggable search integration | 2026-04-08 |
| [[TurboQuant]] | Main upstream method entity that anchors TinyQuant's algorithmic lineage and quality expectations | 2026-04-08 |
| [[PolarQuant]] | Upstream method entity for the random-preconditioning and zero-overhead quantization story | 2026-04-08 |
| [[QJL]] | Upstream method entity for the residual-correction and unbiased-inner-product lineage | 2026-04-08 |

## Concepts

| Page | Summary | Date |
|------|---------|------|
| [[storage-codec-vs-search-backend-separation]] | Core architectural boundary: TinyQuant owns compression and decompression, while search stays external and FP32 | 2026-04-08 |
| [[per-collection-compression-policy]] | Operational concept for deciding which collections should compress, pass through, or use alternative precision modes | 2026-04-08 |
| [[random-preconditioning-without-normalization-overhead]] | Core quantization concept: random transforms can remove normalization overhead and make low-bit storage practical | 2026-04-08 |
| [[two-stage-quantization-and-residual-correction]] | Core quantization concept: stage one captures most structure, stage two protects inner-product fidelity | 2026-04-08 |

## Design

| Page | Summary | Date |
|------|---------|------|
| [[design/domain-layer/README|Domain Layer README]] | Placeholder for future domain-level design analysis and reading order | 2026-04-08 |
| [[storage-codec-architecture]] | Three-layer TinyQuant architecture: codec core, corpus container, and pluggable search backend boundary | 2026-04-08 |

## Behavior Specifications

| Page | Summary | Date |
|------|---------|------|
| [[behavior/README|Behavior README]] | Placeholder for future behavior specs and acceptance-criteria organization | 2026-04-08 |

## Specs

| Page | Summary | Date |
|------|---------|------|
| *No spec pages yet* | | |

## Comparisons

| Page | Summary | Date |
|------|---------|------|
| *No comparison pages yet* | | |

## Raw sources available for ingestion

- `research/llm-wiki.md` — schema and operating model for the TinyQuant
  documentation system
- ~~`research/vector-quantization-paper-synthesis.md`~~ — ingested 2026-04-08
  -> [[vector-quantization-paper-synthesis]],
  [[random-preconditioning-without-normalization-overhead]],
  [[two-stage-quantization-and-residual-correction]], [[TurboQuant]],
  [[PolarQuant]], [[QJL]]
- ~~`research/turbo-quant-deep-research-report.md`~~ — ingested 2026-04-08 ->
  [[turbo-quant-deep-research-report]], [[TurboQuant]], [[TinyQuant]]
- ~~`research/turbo-quant-deep-research-report2.md`~~ — ingested 2026-04-08 ->
  [[turbo-quant-code-availability]], [[TurboQuant]], [[TinyQuant]]
- ~~`research/turboquant-integration.md`~~ — ingested 2026-04-08 ->
  [[turboquant-integration]], [[TinyQuant]],
  [[per-collection-compression-policy]], [[storage-codec-architecture]]
- ~~`research/tinyquant-research/README.md`~~ — ingested 2026-04-08 ->
  [[tinyquant-library-research]], [[TinyQuant]],
  [[storage-codec-vs-search-backend-separation]],
  [[storage-codec-architecture]]
- ~~`research/tinyquant-research/better-router-integration.md`~~ — ingested
  2026-04-08 -> [[tinyquant-better-router-integration]],
  [[per-collection-compression-policy]],
  [[storage-codec-architecture]]
