---
title: TinyQuant
tags:
  - entity
  - project
  - tinyquant
date-created: 2026-04-08
status: active
category: library
source-count: 6
---

# TinyQuant

> [!info] Project entity
> TinyQuant is a proposed CPU-only vector quantization library for embedding
> storage compression.

## What it is

TinyQuant is a clean-room library that compresses high-dimensional
embedding vectors to low-bit representations while preserving useful
similarity scores.

As of Phase 23, the shipping implementation is the Rust workspace
under `rust/`. The original pure-Python + NumPy package has been
demoted to a test-only **reference implementation** under
`tests/reference/tinyquant_py_reference/` — see
[[python-reference-implementation]]. The reference is frozen at the
`v0.1.1` public-surface behavior and its only consumers after Phase 23
are the parity tests under `tests/parity/` and the Rust fixture
generator. The last pure-Python PyPI release remains
`tinyquant-cpu==0.1.1`; Phase 24 reclaims that name with a Rust-backed
fat wheel at `0.2.0+`.

## What it is not

TinyQuant is not positioned as:

- a vector database
- a search engine
- a compressed-domain ANN accelerator

Instead, it is a storage codec that should integrate cleanly with external
search systems and context stores.

## Design posture

- CPU-only by default
- Apache 2.0-friendly clean-room provenance
- strong emphasis on deterministic seeds and reproducible transforms
- decompression-first search boundary
- support for multiple backends through a narrow search protocol

## Planned architecture

- core quantization and residual-correction primitives
- corpus container for batch storage workflows
- pluggable search backend interface
- optional compiled acceleration layer later, validated against the Python
  baseline

## Main downstream integration target

The current research assumes TinyQuant is consumed through TurboSwede in the
better-router stack, where compression is applied on write and retrieval stays
FP32 on the hot path.

## Upstream lineage

The current documentation positions TinyQuant as a downstream adaptation of the
research line represented by [[QJL]], [[PolarQuant]], and especially
[[TurboQuant]].

## Key constraints

- 4-bit mode is the practical default from the current research baseline
- top-1 retrieval degradation grows with corpus size, so exact retrieval should
  not rely on compressed-domain search
- the library's value comes from storage reduction and acceptable score
  fidelity, not from replacing search infrastructure

## See also

- [[python-reference-implementation]]
- [[plans/rust/phase-23-python-reference-demotion|Phase 23: Python Reference Demotion]]
- [[tinyquant-library-research]]
- [[tinyquant-better-router-integration]]
- [[sources/vector-quantization-paper-synthesis|vector-quantization-paper-synthesis]]
- [[sources/turbo-quant-deep-research-report|turbo-quant-deep-research-report]]
- [[sources/turboquant-integration|turboquant-integration]]
- [[TurboQuant]]
- [[storage-codec-vs-search-backend-separation]]
- [[per-collection-compression-policy]]
- [[storage-codec-architecture]]
