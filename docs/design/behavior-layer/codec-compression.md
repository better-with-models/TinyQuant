---
title: Codec Compression Behavior
tags:
  - design
  - behavior
  - bdd
  - codec
  - compression
date-created: 2026-04-08
status: active
category: design
source-count: 6
---

# Codec Compression Behavior

> [!info] Capability
> Compress FP32 embedding vectors into low-bit representations using the
> TinyQuant codec pipeline.

## Actor

Library consumer (application code calling TinyQuant's compression API).

## Preconditions

- A valid `CodecConfig` (bit width, seed, dimension, residual flag)
- A trained `Codebook` matching the config
- One or more FP32 input vectors of the expected dimension

---

## Scenarios

### Compress a single vector at 4-bit

```gherkin
Feature: Codec compression

  Scenario: Compress a single FP32 vector at 4-bit
    Given a CodecConfig with bit_width 4, seed 42, and dimension 768
    And a Codebook trained from representative vectors using that config
    And an FP32 vector of dimension 768
    When the codec compresses the vector
    Then the result is a CompressedVector
    And every index in the CompressedVector is in the range [0, 16)
    And the CompressedVector carries a config_hash matching the CodecConfig
```

### Deterministic compression

```gherkin
  Scenario: Same input and config always produce the same output
    Given a CodecConfig with bit_width 4, seed 42, and dimension 768
    And a trained Codebook
    And an FP32 vector V
    When the codec compresses V twice with the same config and codebook
    Then both CompressedVectors are byte-identical
```

### Compression with residual correction

```gherkin
  Scenario: Residual correction is applied when enabled
    Given a CodecConfig with residual_enabled true
    And a trained Codebook
    And an FP32 vector V
    When the codec compresses V
    Then the CompressedVector includes a non-empty residual field
```

### Compression without residual correction

```gherkin
  Scenario: No residual data when residual correction is disabled
    Given a CodecConfig with residual_enabled false
    And a trained Codebook
    And an FP32 vector V
    When the codec compresses V
    Then the CompressedVector has a null residual field
```

### Dimension mismatch rejection

```gherkin
  Scenario: Reject a vector whose dimension does not match the config
    Given a CodecConfig with dimension 768
    And an FP32 vector of dimension 512
    When the codec attempts to compress the vector
    Then the operation fails with a dimension mismatch error
    And no CompressedVector is produced
```

### Batch compression

```gherkin
  Scenario: Compress a batch of vectors
    Given a CodecConfig and trained Codebook
    And a batch of 1000 FP32 vectors of the correct dimension
    When the codec compresses the batch
    Then 1000 CompressedVectors are returned
    And each carries the same config_hash
```

---

## Rules encoded by these scenarios

1. **Index range:** every quantized index fits within `[0, 2^bit_width)`
2. **Determinism:** identical inputs and config produce identical outputs
3. **Residual presence:** controlled by `residual_enabled`, never ambiguous
4. **Dimension enforcement:** mismatched dimensions are rejected, not silently truncated
5. **Config traceability:** every compressed vector carries a hash linking it to its originating config

## Automation level

These scenarios target the codec's Python API directly. No UI, no network, no
persistence.

## See also

- [[behavior-layer/codec-decompression|Codec Decompression]]
- [[domain-layer/aggregates-and-entities|Aggregates and Entities]]
- [[domain-layer/ubiquitous-language|Ubiquitous Language]]
- [[two-stage-quantization-and-residual-correction]]
