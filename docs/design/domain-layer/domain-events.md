---
title: Domain Events
tags:
  - design
  - domain
  - events
date-created: 2026-04-08
status: active
category: design
source-count: 6
---

# Domain Events

> [!info] Purpose
> Facts that the TinyQuant domain model cares about. These represent meaningful
> state transitions, not generic CRUD notifications.

## Event design principles

- Events are past-tense facts: something that already happened
- Events carry enough context for consumers to act without re-querying
- Events use [[domain-layer/ubiquitous-language|ubiquitous language]] terms
- Events do not leak implementation details (no array indices or memory addresses)

## Codec Context events

### CodebookTrained

Raised when a new codebook has been generated from training vectors.

| Field | Type | Notes |
|-------|------|-------|
| `codec_config` | CodecConfig | The configuration used for training |
| `entry_count` | int | Number of codebook entries (`2^bit_width`) |
| `training_vector_count` | int | How many vectors were used for calibration |
| `timestamp` | datetime | When training completed |

**Why this matters:** downstream corpus creation depends on a valid codebook.
This event marks the moment a codec configuration becomes usable.

### RotationMatrixGenerated

Raised when a rotation matrix is derived from a seed.

| Field | Type | Notes |
|-------|------|-------|
| `seed` | int | The seed used |
| `dimension` | int | Matrix dimension |
| `timestamp` | datetime | When generation completed |

**Why this matters:** determinism verification. Consumers can log this event
to confirm that the same seed always produces the same matrix.

---

## Corpus Context events

### CorpusCreated

Raised when a new corpus is initialized.

| Field | Type | Notes |
|-------|------|-------|
| `corpus_id` | str | Unique corpus identity |
| `codec_config` | CodecConfig | Frozen configuration for this corpus |
| `compression_policy` | str | One of `compress`, `passthrough`, `fp16` |
| `timestamp` | datetime | Creation time |

**Why this matters:** establishes the configuration contract. Every subsequent
vector insertion must conform to this configuration.

### VectorsInserted

Raised after one or more vectors are successfully compressed and stored in a
corpus.

| Field | Type | Notes |
|-------|------|-------|
| `corpus_id` | str | Target corpus |
| `vector_ids` | list[str] | IDs of inserted vectors |
| `count` | int | Number of vectors inserted |
| `timestamp` | datetime | Insertion time |

**Why this matters:** the write path is the critical integration point.
Downstream systems need to know when new compressed
vectors are available.

### CorpusDecompressed

Raised when a batch decompression produces FP32 vectors ready for backend
handoff.

| Field | Type | Notes |
|-------|------|-------|
| `corpus_id` | str | Source corpus |
| `vector_count` | int | Number of vectors decompressed |
| `timestamp` | datetime | Decompression time |

**Why this matters:** marks the transition from compressed storage to the
search-ready read path. Backends should only receive vectors after this event.

### CompressionPolicyViolationDetected

Raised when an operation attempts to violate the corpus's compression policy
or configuration invariants.

| Field | Type | Notes |
|-------|------|-------|
| `corpus_id` | str | Affected corpus |
| `violation_type` | str | e.g. `config_mismatch`, `policy_conflict` |
| `detail` | str | Human-readable description |
| `timestamp` | datetime | Detection time |

**Why this matters:** this is a domain-level error, not just a validation
exception. It indicates a meaningful boundary has been crossed — for example,
attempting to insert a vector compressed with a different seed into an existing
corpus.

---

## Events that TinyQuant does NOT raise

| Non-event | Why not |
|-----------|---------|
| VectorSearched | Search belongs to external backends, not TinyQuant |
| IndexRebuilt | TinyQuant does not own ANN indexes |
| UserAuthenticated | Security is outside TinyQuant's scope |
| ConfigChanged | Configs are value objects; they are created, not changed |

## Event consumption guidance

> [!tip] Integration pattern
> TinyQuant itself is a library, not a message-broker-connected service. These
> events are modeled as in-process callbacks or return values, not as messages
> on a queue. Downstream systems that need async notification should wrap
> TinyQuant's events in their own messaging infrastructure.

## See also

- [[domain-layer/ubiquitous-language|Ubiquitous Language]]
- [[domain-layer/aggregates-and-entities|Aggregates and Entities]]
- [[domain-layer/context-map|Context Map]]
- [[storage-codec-vs-search-backend-separation]]
- [[per-collection-compression-policy]]
