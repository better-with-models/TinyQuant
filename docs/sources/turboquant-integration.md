---
title: TurboQuant Integration Research
tags:
  - source
  - turboquant
  - integration
  - systems
date-created: 2026-04-08
status: active
source-type: research-summary
source-count: 2
---

# TurboQuant Integration Research

> [!info] Source summary
> Summary of `docs/research/turboquant-integration.md`.

## Core claim

TurboQuant has multiple possible integration points across the broader
better-router ecosystem, but the best ROI comes from storage and retrieval
systems before attempting inference-backend KV-cache changes.

## Main findings

- OpenViking and its vector index are the strongest first integration target
- feature caches, gold-corpus storage, and orchestration history also benefit
- KV-cache integration at the inference backend level is high impact but also
  the highest implementation-risk path
- the write path and the read path should be treated differently when designing
  how compression participates in the system

## Why this matters for TinyQuant

This source supports TinyQuant's downstream deployment story:

- TinyQuant should help larger systems compress vectors safely
- the best first integrations are storage-heavy and tolerance-friendly
- project scope should stay away from requiring invasive inference-backend
  changes in the first implementation wave

## See also

- [[TinyQuant]]
- [[TurboQuant]]
- [[per-collection-compression-policy]]
- [[storage-codec-architecture]]
