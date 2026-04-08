---
title: Vector Quantization Paper Synthesis
tags:
  - source
  - research-synthesis
  - turboquant
  - qjl
  - polarquant
date-created: 2026-04-08
status: active
source-type: research-summary
source-count: 3
---

# Vector Quantization Paper Synthesis

> [!info] Source summary
> Summary of `docs/research/vector-quantization-paper-synthesis.md`.

## Core claim

QJL, PolarQuant, and TurboQuant form a coherent research progression around
random preconditioning and low-bit quantization for high-dimensional vectors.

## Main findings

- random preconditioning removes the need for per-block normalization overhead
- TurboQuant's two-stage design resolves the trade-off between MSE distortion
  and inner-product fidelity
- 3-bit KV cache compression is experimentally viable on the cited LLM
  benchmarks
- TurboQuant's data-oblivious quantization makes indexing time effectively
  negligible compared with product-quantization baselines
- exact deployment characteristics still need validation on the target stack

## Why this matters for TinyQuant

This synthesis gives TinyQuant its upstream research lineage:

- [[QJL]] contributes the residual-correction and unbiased-inner-product story
- [[PolarQuant]] contributes the zero-overhead random-preconditioning story
- [[TurboQuant]] combines those ideas into the practical nearest-neighbor and
  KV-cache framing that TinyQuant wants to adapt for CPU-first embedding
  storage

## Main concepts reinforced

- [[random-preconditioning-without-normalization-overhead]]
- [[two-stage-quantization-and-residual-correction]]
- [[storage-codec-vs-search-backend-separation]]

## See also

- [[TurboQuant]]
- [[PolarQuant]]
- [[QJL]]
- [[TinyQuant]]
