---
title: Two Stage Quantization And Residual Correction
tags:
  - concept
  - quantization
  - residuals
date-created: 2026-04-08
status: active
category: concept
source-count: 3
---

# Two Stage Quantization And Residual Correction

> [!question] What problem does this solve?
> A quantizer that is good for MSE is not automatically good for inner-product
> fidelity. The research lineage around TurboQuant addresses that with a
> two-stage design.

## Core idea

Stage 1 captures most of the vector with a low-bit quantization scheme. Stage 2
uses a lightweight residual mechanism, associated in the research set with QJL,
to preserve inner-product behavior more faithfully than the first stage alone.

## Why it matters for TinyQuant

TinyQuant is not trying to be just another byte-packing scheme. It is trying to
preserve useful similarity behavior while still delivering significant storage
savings. This concept is one of the strongest reasons the project is worth
building.

## Scope implication

TinyQuant may choose to stage implementation work carefully, but the
documentation should keep this distinction visible:

- scalar quantization alone is not the whole story
- residual correction is part of the quality claim
- downstream systems will care about inner-product fidelity, not just
  reconstruction error

## See also

- [[QJL]]
- [[TurboQuant]]
- [[TinyQuant]]
