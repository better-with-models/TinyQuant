---
title: Turbo Quant Code Availability
tags:
  - source
  - turboquant
  - implementation
  - provenance
date-created: 2026-04-08
status: active
source-type: research-summary
source-count: 1
---

# Turbo Quant Code Availability

> [!info] Source summary
> Summary of `docs/research/turbo-quant-deep-research-report2.md`.

## Core claim

TurboQuant currently has a weaker official code story than related quantization
 methods, which makes implementation provenance and clean-room discipline
 especially important for TinyQuant.

## Main findings

- most adjacent quantization methods have official author-maintained code
- TurboQuant itself appears to rely mainly on community implementations rather
  than an official Google release
- available implementations vary in language, license, and deployment posture
- some available community implementations use GPL terms that are incompatible
  with TinyQuant's intended downstream positioning

## Why this matters for TinyQuant

This source strengthens the case for:

- a clean-room implementation path
- careful documentation of provenance
- avoiding accidental dependency on community repos whose licenses or runtime
  assumptions do not fit TinyQuant

## See also

- [[TurboQuant]]
- [[TinyQuant]]
- [[random-preconditioning-without-normalization-overhead]]
