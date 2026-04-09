---
title: Corpus Events
tags:
  - classes
  - corpus
  - events
date-created: 2026-04-08
status: active
category: design
---

# Corpus Events

> [!info] Responsibility
> Domain event dataclasses raised by the [[classes/corpus|Corpus]] aggregate
> to signal meaningful state transitions. Modeled as frozen dataclasses
> returned from aggregate methods, not as messages on a queue.

## Location

```text
src/tinyquant_cpu/corpus/events.py
```

## Category

**Value objects** — `@dataclass(frozen=True)`. Events are immutable facts.

---

## CorpusCreated

Raised when a new corpus is initialized.

| Field | Type | Description |
|-------|------|-------------|
| `corpus_id` | `str` | Unique corpus identity |
| `codec_config` | `CodecConfig` | Frozen configuration |
| `compression_policy` | `CompressionPolicy` | Selected write-path policy |
| `timestamp` | `datetime` | UTC creation time |

---

## VectorsInserted

Raised after one or more vectors are successfully compressed and stored.

| Field | Type | Description |
|-------|------|-------------|
| `corpus_id` | `str` | Target corpus |
| `vector_ids` | `tuple[str, ...]` | IDs of inserted vectors (tuple for immutability) |
| `count` | `int` | Number of vectors inserted |
| `timestamp` | `datetime` | UTC insertion time |

---

## CorpusDecompressed

Raised when a batch decompression produces FP32 vectors ready for handoff.

| Field | Type | Description |
|-------|------|-------------|
| `corpus_id` | `str` | Source corpus |
| `vector_count` | `int` | Number of vectors decompressed |
| `timestamp` | `datetime` | UTC decompression time |

---

## CompressionPolicyViolationDetected

Raised when an operation attempts to violate the corpus's configuration or
policy invariants.

| Field | Type | Description |
|-------|------|-------------|
| `corpus_id` | `str` | Affected corpus |
| `violation_type` | `str` | Category: `config_mismatch`, `policy_conflict`, `duplicate_id` |
| `detail` | `str` | Human-readable description |
| `timestamp` | `datetime` | UTC detection time |

---

## Common interface

All events share:

- `@dataclass(frozen=True)` — immutable
- A `timestamp` field — UTC `datetime`
- A `corpus_id` field — links the event to its aggregate

## Consumption pattern

Events are returned from Corpus methods, not broadcast. Consumers collect
them from method return values or a post-operation event list:

```python
corpus = Corpus(...)
# CorpusCreated is available after construction
entry = corpus.insert("vec-001", vector)
# VectorsInserted is available after insert
events = corpus.pending_events()  # collect and clear
```

> [!tip] Integration
> Downstream systems that need async delivery should wrap these events in
> their own messaging infrastructure. TinyQuant does not own a message bus.

## Test file

```text
tests/corpus/test_events.py
```

## See also

- [[classes/corpus|Corpus]]
- [[design/domain-layer/domain-events|Domain Events]]
