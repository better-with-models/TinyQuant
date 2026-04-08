---
title: Domain Layer
tags:
  - design
  - domain
  - meta
date-created: 2026-04-08
status: active
category: design
---

# Domain Layer

> [!info] Purpose
> Domain-level analysis for TinyQuant using domain-driven design. This folder
> defines bounded contexts, aggregates, ubiquitous language, invariants, and
> domain events that shape the library's internal model.

## Reading order

1. [[domain-layer/ubiquitous-language|Ubiquitous Language]] — shared vocabulary
2. [[domain-layer/context-map|Context Map]] — bounded contexts and their relationships
3. [[domain-layer/aggregates-and-entities|Aggregates and Entities]] — consistency boundaries and lifecycle objects
4. [[domain-layer/domain-events|Domain Events]] — facts the system cares about

## Design posture

TinyQuant is a focused codec library, not an enterprise application. The DDD
analysis here is deliberately lightweight: just enough structure to protect
important invariants and keep the domain language consistent, without
over-engineering a library that should stay small and composable.

> [!warning] Not microservices
> The bounded contexts described here are **module boundaries inside a single
> library**, not deployment units. TinyQuant ships as one Python package.

## See also

- [[storage-codec-architecture]]
- [[storage-codec-vs-search-backend-separation]]
- [[TinyQuant]]
