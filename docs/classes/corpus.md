---
title: Corpus
tags:
  - classes
  - corpus
  - aggregate-root
date-created: 2026-04-08
status: active
category: design
---

# Corpus

> [!info] Responsibility
> Aggregate root for stored compressed vectors. Enforces configuration
> consistency, compression policy, and vector lifecycle within a single
> collection.

## Location

```text
src/tinyquant_cpu/corpus/corpus.py
```

## Category

**Aggregate root** — the primary consistency boundary. All vector mutations
flow through this class.

## Constructor

| Parameter | Type | Description |
|-----------|------|-------------|
| `corpus_id` | `str` | Unique identity for this corpus |
| `codec_config` | `CodecConfig` | Frozen configuration for all vectors in this corpus |
| `codebook` | `Codebook` | Frozen codebook matching the config |
| `compression_policy` | `CompressionPolicy` | Governs write-path behavior |
| `metadata` | `dict[str, Any]` | Optional per-corpus metadata (default: empty) |

## Fields

| Field | Type | Mutability | Invariant |
|-------|------|-----------|-----------|
| `corpus_id` | `str` | Immutable | Non-empty, unique |
| `codec_config` | `CodecConfig` | Immutable | Frozen at construction |
| `codebook` | `Codebook` | Immutable | `codebook.bit_width == codec_config.bit_width` |
| `compression_policy` | `CompressionPolicy` | Immutable after first insert | Cannot change once vectors exist |
| `metadata` | `dict[str, Any]` | Mutable | Consumer-defined; no structural constraints |
| `_vectors` | `dict[str, VectorEntry]` | Internal | Keyed by `vector_id` |
| `_codec` | `Codec` | Internal | Stateless service instance for compression operations |

## Properties

| Property | Return type | Description |
|----------|-------------|-------------|
| `vector_count` | `int` | Number of stored vector entries |
| `is_empty` | `bool` | `self.vector_count == 0` |
| `vector_ids` | `frozenset[str]` | Set of all stored vector IDs |

## Methods

### `insert`

```python
def insert(
    self,
    vector_id: str,
    vector: NDArray[np.float32],
    metadata: dict[str, Any] | None = None,
) -> VectorEntry:
```

**Purpose:** Compress and store a single vector.

**Behavior by policy:**
- `compress`: full codec pipeline, stores `CompressedVector`
- `passthrough`: stores the FP32 vector as-is (wrapped in a pass-through representation)
- `fp16`: downcasts to FP16, stores without codec quantization

**Preconditions:**
- `vector` dimension matches `codec_config.dimension`
- `vector_id` is not already present in the corpus

**Postconditions:**
- A `VectorEntry` is added to `_vectors`
- The entry's `config_hash` matches `codec_config.config_hash`

**Raises:** `DimensionMismatchError`, `DuplicateVectorError`

**Events:** contributes to a batched `VectorsInserted` event

---

### `insert_batch`

```python
def insert_batch(
    self,
    vectors: dict[str, NDArray[np.float32]],
    metadata: dict[str, dict[str, Any]] | None = None,
) -> list[VectorEntry]:
```

**Purpose:** Compress and store multiple vectors atomically.

**Postconditions:** either all vectors are inserted or none are (transactional).

**Events:** raises `VectorsInserted` with all inserted IDs.

---

### `get`

```python
def get(self, vector_id: str) -> VectorEntry:
```

**Purpose:** Retrieve a single vector entry by ID.

**Raises:** `KeyError` if not found.

---

### `decompress_all`

```python
def decompress_all(self) -> dict[str, NDArray[np.float32]]:
```

**Purpose:** Batch decompress all stored vectors to FP32.

**Postconditions:** returns a dict mapping `vector_id` to FP32 arrays.

**Events:** raises `CorpusDecompressed`.

---

### `decompress`

```python
def decompress(self, vector_id: str) -> NDArray[np.float32]:
```

**Purpose:** Decompress a single stored vector to FP32.

**Raises:** `KeyError` if not found.

---

### `contains`

```python
def contains(self, vector_id: str) -> bool:
```

**Purpose:** Check if a vector ID exists in the corpus.

---

### `remove`

```python
def remove(self, vector_id: str) -> None:
```

**Purpose:** Remove a vector entry from the corpus.

**Raises:** `KeyError` if not found.

## Invariants

1. **Config frozen:** `codec_config` cannot change after construction
2. **Codebook frozen:** `codebook` cannot change after construction
3. **Policy immutable after insert:** `compression_policy` cannot be changed once `_vectors` is non-empty
4. **Config consistency:** every stored vector's `config_hash` matches `codec_config.config_hash`
5. **Unique IDs:** no two entries share the same `vector_id`
6. **Transactional batch:** `insert_batch` is all-or-nothing

## Relationships

| Direction | Class | Nature |
|-----------|-------|--------|
| Contains | [[classes/vector-entry\|VectorEntry]] | Owned entities within the aggregate |
| Uses | [[classes/codec-config\|CodecConfig]] | Frozen configuration |
| Uses | [[classes/codebook\|Codebook]] | Frozen codebook |
| Uses | [[classes/compression-policy\|CompressionPolicy]] | Write-path behavior selector |
| Uses | [[classes/codec\|Codec]] | Delegates compression/decompression |
| Produces | [[classes/corpus-events\|CorpusCreated]] | At construction |
| Produces | [[classes/corpus-events\|VectorsInserted]] | After insert/insert_batch |
| Produces | [[classes/corpus-events\|CorpusDecompressed]] | After decompress_all |
| Consumed by | [[classes/search-backend\|SearchBackend]] | Decompressed vectors handed off for search |

## Example

```python
corpus = Corpus(
    corpus_id="gold-embeddings",
    codec_config=config,
    codebook=codebook,
    compression_policy=CompressionPolicy.COMPRESS,
)
entry = corpus.insert("vec-001", vector)
assert corpus.vector_count == 1
fp32 = corpus.decompress("vec-001")
```

## Test file

```text
tests/corpus/test_corpus.py
```

## See also

- [[classes/vector-entry|VectorEntry]]
- [[classes/compression-policy|CompressionPolicy]]
- [[classes/corpus-events|Corpus Events]]
- [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]]
