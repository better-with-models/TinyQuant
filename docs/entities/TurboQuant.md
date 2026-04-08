---
title: TurboQuant
tags:
  - entity
  - method
  - turboquant
date-created: 2026-04-08
status: active
category: upstream-method
source-count: 4
---

# TurboQuant

> [!info] Upstream method
> TurboQuant is the main upstream algorithmic reference for TinyQuant.

## What it is

TurboQuant is a two-stage vector compression method aimed at KV-cache
compression and high-dimensional vector applications. In the current TinyQuant
research set, it acts as the main source of algorithmic and systems inspiration.

## Main importance for TinyQuant

- gives TinyQuant its strongest upstream algorithmic identity
- motivates the storage-codec framing rather than a pure ANN framing
- provides the quality and compression baselines the project should validate
  against on CPU-first workloads

## Key lineage

TurboQuant builds on ideas associated with [[PolarQuant]] and [[QJL]] and is
the most directly relevant upstream reference for TinyQuant's intended design.

## Practical caution

The research set also highlights that:

- official-code availability is weaker than for many neighboring methods
- published headline benefits should be validated locally before being treated
  as project commitments
- some community implementations are not license-compatible with TinyQuant's
  intended downstream uses

## See also

- [[TinyQuant]]
- [[PolarQuant]]
- [[QJL]]
- [[two-stage-quantization-and-residual-correction]]
