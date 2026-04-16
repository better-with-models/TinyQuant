---
title: TinyQuant Functional Requirements
tags:
  - requirements
  - planguage
date-created: 2026-04-15
status: draft
category: requirements
---

# TinyQuant Functional Requirements

> [!info] Purpose
> Measurable functional requirements for TinyQuant written in Planguage
> (Tom Gilb's Planning Language). Each requirement is tagged, scoped to
> a domain, and carries a meter so any engineer can verify compliance
> without ambiguity.

## What Planguage fields mean here

| Field | Meaning |
|---|---|
| `Gist` | One-line plain-English summary |
| `Type` | `Function` — a behavioral requirement; `Constraint` — a hard limit |
| `Function` | The precise behavioral statement the system shall satisfy |
| `Actor` | The party that invokes or depends on this requirement |
| `Scale` | The attribute being measured for compliance |
| `Meter` | Concrete test procedure that produces the Scale value |
| `Must` | Mandatory threshold — below this the system fails acceptance |
| `Plan` | Target the implementation is expected to reach |
| `Stretch` | Aspirational level; not a release blocker |
| `Past` | Baseline from the Python reference implementation |
| `Qualify` | Conditions under which this requirement applies |
| `Rationale` | Why the requirement exists |
| `Tests` | Known test coverage — `[Rust]` or `[Python]` or `[CI]` prefixed paths |
| `Gap` | Missing coverage with a `GAP-<DOMAIN>-<NNN>` tag; "None." if fully covered |

## Requirement files

| File | Domain | Tags |
|---|---|---|
| [codec.md](codec.md) | Compression, decompression, serialization | FR-COMP-*, FR-DECOMP-*, FR-SER-* |
| [corpus.md](corpus.md) | Corpus management and compression policy | FR-CORP-* |
| [backend.md](backend.md) | Backend protocol and adapter contracts | FR-BACK-* |
| [quality.md](quality.md) | Score fidelity and compression ratio | FR-QUAL-* |
| [performance.md](performance.md) | Latency and throughput budgets | FR-PERF-* |
| [gpu.md](gpu.md) | Optional GPU acceleration layer | FR-GPU-* |
| [js.md](js.md) | JavaScript / TypeScript N-API binding | FR-JS-* |
| [testing-gaps.md](testing-gaps.md) | Prioritized list of all `Gap:` entries across requirements | GAP-* |

## Tag numbering convention

```
FR-<DOMAIN>-<NNN>
```

- `FR` — Functional Requirement
- `<DOMAIN>` — `COMP`, `DECOMP`, `SER`, `CORP`, `BACK`, `QUAL`, `PERF`, `GPU`, `JS`
- `<NNN>` — three-digit sequential number within domain

Gaps in numbering are intentional. Reserved numbers are not filled.

## Authority

All requirements are owned by the codec lead unless stated otherwise.
The Python reference implementation (`tinyquant_cpu` v0.1.1) is the
behavioral oracle: a requirement whose `Must` level cannot be met by
the Python reference is invalid and must be revised before acceptance.

## See also

- [[design/behavior-layer/README|Behavior Layer]]
- [[design/rust/goals-and-non-goals|Goals and Non-Goals]]
- [[design/domain-layer/ubiquitous-language|Ubiquitous Language]]
