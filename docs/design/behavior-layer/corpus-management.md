---
title: Corpus Management Behavior
tags:
  - design
  - behavior
  - bdd
  - corpus
date-created: 2026-04-08
status: active
category: design
source-count: 6
---

# Corpus Management Behavior

> [!info] Capability
> Create, populate, and manage corpora of compressed vectors with enforced
> configuration consistency and per-collection compression policies.

## Actor

Downstream consumer (application code integrating TinyQuant).

## Preconditions

- A valid CodecConfig and trained Codebook
- A chosen CompressionPolicy

---

## Scenarios

### Create a corpus with a compression policy

```gherkin
Feature: Corpus management

  Scenario: Create a new corpus with the "compress" policy
    Given a CodecConfig with bit_width 4 and seed 42
    And a trained Codebook
    When the consumer creates a Corpus with compression_policy "compress"
    Then the Corpus is created with a unique corpus_id
    And the CodecConfig is frozen on the corpus
    And the compression_policy is "compress"
```

### Insert vectors into a corpus

```gherkin
  Scenario: Insert vectors into an existing corpus
    Given a Corpus with compression_policy "compress"
    And 100 FP32 vectors of the correct dimension
    When the consumer inserts the vectors into the corpus
    Then the corpus contains 100 VectorEntries
    And each entry's config_hash matches the corpus's CodecConfig
```

### Reject cross-config insertion

```gherkin
  Scenario: Reject a vector compressed under a different config
    Given a Corpus created with seed 42
    And a CompressedVector produced with seed 99
    When the consumer attempts to insert that vector
    Then the operation fails with a config mismatch error
    And the corpus remains unchanged
```

### Passthrough policy stores FP32

```gherkin
  Scenario: Passthrough policy stores vectors without compression
    Given a Corpus with compression_policy "passthrough"
    And an FP32 vector V
    When the consumer inserts V into the corpus
    Then V is stored as-is in FP32 format
    And no CompressedVector is created
```

### FP16 policy downcasts only

```gherkin
  Scenario: FP16 policy stores vectors at half precision
    Given a Corpus with compression_policy "fp16"
    And an FP32 vector V
    When the consumer inserts V into the corpus
    Then V is stored in FP16 format
    And no codec quantization is applied
```

### Policy immutability after first insertion

```gherkin
  Scenario: Cannot change compression policy on a populated corpus
    Given a Corpus with compression_policy "compress" and 10 stored vectors
    When the consumer attempts to change the policy to "passthrough"
    Then the operation fails with a policy immutability error
    And the corpus retains its original policy and vectors
```

### Batch decompression for backend handoff

```gherkin
  Scenario: Batch decompress a corpus for search backend consumption
    Given a Corpus containing 500 compressed vectors
    When the consumer requests batch decompression
    Then 500 FP32 vectors are returned
    And a CorpusDecompressed event is raised
```

### Empty corpus creation

```gherkin
  Scenario: Create an empty corpus for future population
    Given a valid CodecConfig and Codebook
    When the consumer creates a Corpus with compression_policy "compress"
    Then the corpus exists with zero vectors
    And is ready to accept insertions
```

---

## Rules encoded by these scenarios

1. **Config freezing:** a corpus's CodecConfig is immutable after creation
2. **Policy enforcement:** the compression policy governs write-path behavior
3. **Policy immutability:** policy cannot change once vectors exist
4. **Cross-config rejection:** vectors from a different config are refused
5. **Policy variants:** `compress`, `passthrough`, and `fp16` each have distinct storage behavior
6. **Event signaling:** batch decompression raises a domain event

## Automation level

Python API tests against the Corpus class. Policy enforcement scenarios are
pure domain logic with no I/O.

## See also

- [[behavior-layer/codec-compression|Codec Compression]]
- [[behavior-layer/codec-decompression|Codec Decompression]]
- [[domain-layer/aggregates-and-entities|Aggregates and Entities]]
- [[per-collection-compression-policy]]
