---
title: TinyQuant Library Research
tags:
  - source
  - tinyquant
  - compression
date-created: 2026-04-08
status: active
source-type: research-summary
source-count: 1
---

# TinyQuant Library Research

> [!info] Source summary
> Summary of `docs/research/tinyquant-research/README.md`.

## Core claim

TinyQuant is intended to be a CPU-only, clean-room vector quantization library
 that compresses embedding vectors to 2 to 4 bits with high score fidelity while
 remaining a **storage codec** rather than a compressed-domain search engine.

## Key takeaways

- The target implementation is a pure Python and NumPy package with a future
  compiled core path
- The algorithm combines PolarQuant-style rotation, Lloyd-Max scalar
  quantization, and optional QJL residual correction
- The library is aimed at storage-heavy embedding systems that still want
  search to happen on decompressed FP32 vectors
- The design goal is Apache 2.0 clean-room provenance, explicitly avoiding GPL
  contamination from community GPU implementations
- Experimental results from better-router E37 and E38 are the baseline quality
  contract for TinyQuant

## Quantitative baseline

- 4-bit mode is the default quality-compression tradeoff
- Compression target is roughly 8x at common embedding dimensions
- Gold score fidelity target is near `Pearson rho = 0.997`
- Rank correlation remains high enough for approximate neighborhood tasks, but
  exact top-1 retrieval degrades with corpus size

## Architecture summary

The research proposes three layers:

1. Core codec primitives such as codebook generation, quantization, residual
   correction, and vector serialization
2. A corpus container for batch compression, decompression, and persistence
3. A pluggable search backend interface that operates on decompressed FP32
   vectors

## Implications for TinyQuant

- [[TinyQuant]] should be documented and positioned as a reusable compression
  library, not as a vector database
- [[storage-codec-vs-search-backend-separation]] is a core design principle
- [[per-collection-compression-policy]] matters downstream because different
  workloads tolerate different levels of fidelity loss

## See also

- [[TinyQuant]]
- [[storage-codec-vs-search-backend-separation]]
- [[per-collection-compression-policy]]
- [[storage-codec-architecture]]
